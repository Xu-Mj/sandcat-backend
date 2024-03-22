mod client;
mod manager;
mod rpc;

use std::sync::Arc;
use std::time::Duration;

use crate::client::Client;
use crate::rpc::MsgRpcService;
use abi::config::Config;
use abi::msg::msg_service_server::MsgServiceServer;
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{
    extract::ws::{Message, WebSocket},
    Router,
};
use futures::{SinkExt, StreamExt};
use kafka::producer::{Producer, RequiredAcks};
use tokio::sync::{mpsc, RwLock};
use tonic::transport::Server;
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
        let redis = redis::Client::open(config.redis.url()).expect("redis can't open");
        let producer = Producer::from_hosts(config.kafka.hosts)
            // ~ give the brokers one second time to ack the message
            .with_ack_timeout(Duration::from_secs(1))
            // ~ require only one broker to ack the message
            .with_required_acks(RequiredAcks::One)
            // ~ build the producer with the above settings
            .create()
            .expect("Producer creation error");
        let hub = Manager::new(tx, redis, producer);
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
                "/:user_id/conn/:token/:pointer_id",
                get(Self::websocket_handler),
            )
            .with_state(app_state);
        let addr = "127.0.0.1:8000";
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        tracing::debug!("listening on {}", listener.local_addr().unwrap());
        let mut ws = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let mut rpc = tokio::spawn(async move {
            // start rpc server
            let addr = "127.0.0.1:8001";
            let service = MsgRpcService::new(hub);
            let svc = MsgServiceServer::new(service);
            Server::builder()
                .add_service(svc)
                .serve(addr.parse().unwrap())
                .await
                .unwrap();
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
        // 验证token
        tracing::debug!("token is {}", token);
        ws.on_upgrade(move |socket| Self::websocket(user_id, pointer_id, socket, state))
    }

    pub async fn websocket(
        user_id: String,
        pointer_id: String,
        ws: WebSocket,
        app_state: AppState,
    ) {
        // 注册客户端
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
        // todo mark user online
        // if let Err(err) = update_online(&state.pg_pool, &user_id, true).await {
        //     tracing::error!("mark user online error: {}", err);
        // }

        // drop(guard);
        tracing::debug!("offline msg sync done");
        // 开启任务发送心跳,这里直接回发即可
        let cloned_tx = shared_tx.clone();
        let mut ping_task = tokio::spawn(async move {
            loop {
                if let Err(e) = cloned_tx
                    .write()
                    .await
                    .send(Message::Ping(Vec::new()))
                    .await
                {
                    // 这里其实是需要处理的，后面再看看有没有别的方式吧？
                    tracing::error!("心跳发送失败：{:?}", e);
                    break;
                } else {
                    // tracing::debug!("心跳发送成功");
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(HEART_BEAT_INTERVAL)).await;
            }
        });
        let cloned_hub = hub.clone();
        let shared_tx = shared_tx.clone();
        // 读取收到的消息
        let mut rec_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_rx.next().await {
                // 处理消息
                match msg {
                    Message::Text(text) => {
                        let result = serde_json::from_str(&text);
                        if result.is_err() {
                            error!("反序列化错误: {:?}； source: {text}", result.err());
                            continue;
                        }

                        if cloned_hub.broadcast(result.unwrap()).await.is_err() {
                            // 如果广播出错，那么服务端服务不可用
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
                            error!("心跳回复失败 : {:?}", e);
                            break;
                        }
                    }
                    Message::Pong(_) => {
                        // tracing::debug!("收到心跳回复消息");
                    }
                    Message::Close(info) => {
                        if let Some(info) = info {
                            tracing::warn!("client closed {}", info.reason);
                        }
                        // todo mark client offline
                        // if let Err(e) = update_online(&state.pg_pool, &cloned_user_id, false).await {
                        //     tracing::error!("更新用户在线状态失败: {}", e);
                        // }
                        break;
                    }

                    Message::Binary(_) => {}
                }
            }
        });

        tokio::select! {
            _ = (&mut ping_task) => rec_task.abort(),
            _ = (&mut rec_task) => ping_task.abort(),
        }
        hub.unregister(user_id, pointer_id).await;
        tracing::debug!("client thread exit {}", hub.hub.iter().count());
    }
}
