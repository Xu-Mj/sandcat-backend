use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::CloseFrame;
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{
    extract::ws::{Message, WebSocket},
    Router,
};
use futures::{SinkExt, StreamExt};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tonic::transport::Channel;
use tracing::{error, info, warn};

use abi::config::Config;
use abi::errors::Error;
use abi::message::{Msg, PlatformType};
use synapse::service::{Scheme, ServiceInstance, ServiceRegistryClient};

use crate::client::Client;
use crate::manager::Manager;
use crate::rpc::MsgRpcService;

pub const HEART_BEAT_INTERVAL: u64 = 30;
pub const KNOCK_OFF_CODE: u16 = 4001;
pub const UNAUTHORIZED_CODE: u16 = 4002;

#[derive(Clone)]
pub struct AppState {
    manager: Manager,
    jwt_secret: String,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
}

pub struct WsServer;

impl WsServer {
    async fn register_service(config: &Config) -> Result<(), Error> {
        // register service to service register center
        let addr = format!(
            "{}://{}:{}",
            config.service_center.protocol, config.service_center.host, config.service_center.port
        );
        let channel = Channel::from_shared(addr).unwrap().connect().await.unwrap();
        let mut client = ServiceRegistryClient::new(channel);
        let service = ServiceInstance {
            id: format!("{}-{}", utils::get_host_name()?, &config.websocket.name),
            name: config.websocket.name.clone(),
            address: config.websocket.host.clone(),
            port: config.websocket.port as i32,
            tags: config.websocket.tags.clone(),
            version: "".to_string(),
            metadata: Default::default(),
            health_check: None,
            status: 0,
            scheme: Scheme::from(config.rpc.db.protocol.as_str()) as i32,
        };
        client.register_service(service).await.unwrap();
        Ok(())
    }

    async fn test(State(state): State<AppState>) -> Result<String, Error> {
        let mut description = String::new();

        state.manager.hub.iter().for_each(|entry| {
            let user_id = entry.key();
            let platforms = entry.value();
            description.push_str(&format!("UserID: {}\n", user_id));
            platforms.iter().for_each(|platform_entry| {
                let platform_type = platform_entry.key();
                let client = platform_entry.value();
                description.push_str(&format!(
                    "  Platform: {:?}, PlatformID: {}\n",
                    platform_type, client.platform_id
                ));
            });
        });
        Ok(description)
    }

    pub async fn start(config: Config) {
        let (tx, rx) = mpsc::channel(1024);
        let hub = Manager::new(tx, &config).await;
        let mut cloned_hub = hub.clone();
        tokio::spawn(async move {
            cloned_hub.run(rx).await;
        });
        let app_state = AppState {
            manager: hub.clone(),
            jwt_secret: config.server.jwt_secret.clone(),
        };

        // run axum server
        let router = Router::new()
            .route(
                "/ws/:user_id/conn/:pointer_id/:platform/:token",
                get(Self::websocket_handler),
            )
            .route("/test", get(Self::test))
            .with_state(app_state);
        let addr = format!("{}:{}", config.websocket.host, config.websocket.port);

        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        let mut ws = tokio::spawn(async move {
            info!("start websocket server on {}", addr);
            axum::serve(listener, router).await.unwrap();
        });

        // register websocket service to consul
        Self::register_service(&config).await.unwrap();

        let config = config.clone();
        let mut rpc = tokio::spawn(async move {
            // start rpc server
            MsgRpcService::start(hub, &config).await.unwrap();
        });
        tokio::select! {
            _ = (&mut ws) => ws.abort(),
            _ = (&mut rpc) => rpc.abort(),
        }
    }

    fn verify_token(token: String, jwt_secret: &String) -> Result<(), Error> {
        if let Err(err) = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(jwt_secret.as_bytes()),
            &Validation::default(),
        ) {
            return Err(Error::unauthorized(err, "/ws"));
        }
        Ok(())
    }

    pub async fn websocket_handler(
        Path((user_id, pointer_id, platform, token)): Path<(String, String, i32, String)>,
        ws: WebSocketUpgrade,
        State(state): State<AppState>,
    ) -> impl IntoResponse {
        let platform = PlatformType::try_from(platform).unwrap_or_default();
        ws.on_upgrade(move |socket| {
            Self::websocket(user_id, pointer_id, token, platform, socket, state)
        })
    }

    pub async fn websocket(
        user_id: String,
        pointer_id: String,
        token: String,
        platform: PlatformType,
        ws: WebSocket,
        app_state: AppState,
    ) {
        tracing::info!(
            "client {} connected, user id : {}",
            user_id.clone(),
            pointer_id.clone()
        );
        let (mut ws_tx, mut ws_rx) = ws.split();
        // validate token
        if let Err(err) = Self::verify_token(token, &app_state.jwt_secret) {
            warn!("verify token error: {:?}", err);
            if let Err(e) = ws_tx
                .send(Message::Close(Some(CloseFrame {
                    code: UNAUTHORIZED_CODE,
                    reason: Cow::Owned("knock off".to_string()),
                })))
                .await
            {
                error!("send verify failed to client error: {}", e);
            }
            return;
        }
        let shared_tx = Arc::new(RwLock::new(ws_tx));
        let (notify_sender, mut notify_receiver) = tokio::sync::mpsc::channel(1);
        let mut hub = app_state.manager.clone();
        let client = Client {
            user_id: user_id.clone(),
            platform_id: pointer_id.clone(),
            sender: shared_tx.clone(),
            platform,
            notify_sender,
        };
        hub.register(user_id.clone(), client).await;

        // send ping message to client
        let cloned_tx = shared_tx.clone();
        let mut ping_task = tokio::spawn(async move {
            loop {
                if let Err(e) = cloned_tx
                    .write()
                    .await
                    .send(Message::Ping(Vec::new()))
                    .await
                {
                    error!("send ping error：{:?}", e);
                    // break this task, it will end this conn
                    break;
                }
                tokio::time::sleep(Duration::from_secs(HEART_BEAT_INTERVAL)).await;
            }
        });

        let shared_clone = shared_tx.clone();
        // watch knock off signal
        let mut watch_task = tokio::spawn(async move {
            if notify_receiver.recv().await.is_none() {
                info!("client {} knock off", pointer_id);
                // send knock off signal to ws server
                if let Err(e) = shared_clone
                    .write()
                    .await
                    .send(Message::Close(Some(CloseFrame {
                        code: KNOCK_OFF_CODE,
                        reason: Cow::Owned("knock off".to_string()),
                    })))
                    .await
                {
                    error!("send knock off signal to client error: {}", e);
                }
            }
        });

        // spawn a new task to receive message
        let cloned_hub = hub.clone();
        let shared_tx = shared_tx.clone();
        // receive message from client
        let mut rec_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_rx.next().await {
                // 处理消息
                match msg {
                    Message::Text(text) => {
                        let result = serde_json::from_str(&text);
                        if result.is_err() {
                            error!("deserialize error: {:?}； source: {text}", result.err());
                            continue;
                        }

                        if cloned_hub.broadcast(result.unwrap()).await.is_err() {
                            // if broadcast not available, close the connection
                            break;
                        }
                    }
                    Message::Ping(_) => {
                        if let Err(e) = shared_tx
                            .write()
                            .await
                            .send(Message::Pong(Vec::new()))
                            .await
                        {
                            error!("reply ping error : {:?}", e);
                            break;
                        }
                    }
                    Message::Pong(_) => {
                        // tracing::debug!("received pong message");
                    }
                    Message::Close(info) => {
                        if let Some(info) = info {
                            warn!("client closed {}", info.reason);
                        }
                        break;
                    }
                    Message::Binary(b) => {
                        let result = bincode::deserialize(&b);
                        if result.is_err() {
                            error!("deserialize error: {:?}； source: {:?}", result.err(), b);
                            continue;
                        }
                        let msg: Msg = result.unwrap();
                        // todo need to judge the local id is empty by message type
                        // if msg.local_id.is_empty() {
                        //     warn!("receive empty message");
                        //     continue;
                        // }
                        if cloned_hub.broadcast(msg).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let mut need_unregister = true;
        tokio::select! {
            _ = (&mut ping_task) => {rec_task.abort(); watch_task.abort();},
            _ = (&mut watch_task) => {need_unregister = false; rec_task.abort(); ping_task.abort();},
            _ = (&mut rec_task) => {ping_task.abort(); watch_task.abort();},
        }

        // lost the connection, remove the client from hub
        if need_unregister {
            hub.unregister(user_id, platform).await;
        }
        tracing::debug!("client thread exit {}", hub.hub.iter().count());
    }
}
