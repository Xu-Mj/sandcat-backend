use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Json;
use axum::{
    extract::ws::{Message, WebSocket},
    Router,
};
use futures::{SinkExt, StreamExt};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use synapse::service::{Scheme, ServiceInstance, ServiceRegistryClient};
use tokio::sync::{mpsc, RwLock};
use tonic::transport::Channel;
use tracing::{error, info};

use abi::config::Config;
use abi::errors::Error;
use abi::message::Msg;

use crate::client::Client;
use crate::manager::Manager;
use crate::rpc::MsgRpcService;

pub const HEART_BEAT_INTERVAL: u64 = 30;

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
        // 直接启动axum server
        let router = Router::new()
            .route(
                "/ws/:user_id/conn/:token/:pointer_id",
                get(Self::websocket_handler),
            )
            .with_state(app_state);
        let addr = format!("{}:{}", config.websocket.host, config.websocket.port);

        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        tracing::debug!("listening on {}", listener.local_addr().unwrap());
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
            return Err(Error::UnAuthorized(err.to_string(), "/ws".to_string()));
        }
        Ok(())
    }

    pub async fn websocket_handler(
        Path((user_id, token, pointer_id)): Path<(String, String, String)>,
        ws: WebSocketUpgrade,
        State(state): State<AppState>,
    ) -> impl IntoResponse {
        // validate token
        if let Err(err) = Self::verify_token(token, &state.jwt_secret) {
            let error_response = Json({
                let error_msg = format!("Token verification failed: {}", err);
                error!("{}", error_msg);
                serde_json::json!({ "error": error_msg })
            });
            return (StatusCode::UNAUTHORIZED, error_response).into_response();
        }

        ws.on_upgrade(move |socket| Self::websocket(user_id, pointer_id, socket, state))
    }

    pub async fn websocket(
        user_id: String,
        pointer_id: String,
        ws: WebSocket,
        app_state: AppState,
    ) {
        tracing::info!(
            "client {} connected, user id : {}",
            user_id.clone(),
            pointer_id.clone()
        );
        let mut hub = app_state.manager.clone();
        let (ws_tx, mut ws_rx) = ws.split();
        let shared_tx = Arc::new(RwLock::new(ws_tx));
        let client = Client {
            user_id: user_id.clone(),
            platform_id: pointer_id.clone(),
            sender: shared_tx.clone(),
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
                            tracing::warn!("client closed {}", info.reason);
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

        tokio::select! {
            _ = (&mut ping_task) => rec_task.abort(),
            _ = (&mut rec_task) => ping_task.abort(),
        }

        // lost the connection, remove the client from hub
        hub.unregister(user_id, pointer_id).await;
        tracing::debug!("client thread exit {}", hub.hub.iter().count());
    }
}
