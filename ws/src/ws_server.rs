use std::sync::Arc;
use std::time::Duration;

use crate::client::Client;
use crate::rpc::MsgRpcService;
use abi::config::Config;
use abi::message::Msg;
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{
    extract::ws::{Message, WebSocket},
    Router,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, RwLock};
use tracing::error;
use utils::custom_extract::path_extractor::PathExtractor;

use crate::manager::Manager;

pub const HEART_BEAT_INTERVAL: u64 = 30;

#[derive(Clone)]
pub struct AppState {
    manager: Manager,
}

/// need mq connection pool
/// redis connection pool
/// client connection pool
pub struct WsServer {
    // client connection pool
    // manager: Manager,
}

impl WsServer {
    pub async fn start(config: Config) {
        let (tx, rx) = mpsc::channel(1024);
        let hub = Manager::new(tx, &config).await;
        let mut cloned_hub = hub.clone();
        tokio::spawn(async move {
            cloned_hub.run(rx).await;
        });
        let app_state = AppState {
            manager: hub.clone(),
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
            println!("start websocket server on {}", addr);
            axum::serve(listener, router).await.unwrap();
        });
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

    pub async fn websocket_handler(
        PathExtractor((user_id, token, pointer_id)): PathExtractor<(String, String, String)>,
        ws: WebSocketUpgrade,
        State(state): State<AppState>,
    ) -> impl IntoResponse {
        // validate token
        tracing::debug!("token is {}", token);
        ws.on_upgrade(move |socket| Self::websocket(user_id, pointer_id, socket, state))
    }

    pub async fn websocket(
        user_id: String,
        pointer_id: String,
        ws: WebSocket,
        app_state: AppState,
    ) {
        tracing::debug!(
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
