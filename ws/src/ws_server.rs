use std::sync::Arc;
use std::time::Duration;

use crate::client::Client;
use crate::rpc::MsgRpcService;
use abi::config::Config;
use abi::message::chat_service_client::ChatServiceClient;
use abi::message::msg_service_server::MsgServiceServer;
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{
    extract::ws::{Message, WebSocket},
    Router,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, RwLock};
use tokio::time;
use tonic::transport::{Channel, Server};
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

async fn get_client(config: &Config) -> ChatServiceClient<Channel> {
    // start server at first
    let url = config.rpc.chat.url(false);

    println!("connect to {}", url);
    // try to connect to server
    let future = async move {
        while ChatServiceClient::connect(url.clone()).await.is_err() {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
        // return result
        ChatServiceClient::connect(url).await.unwrap()
    };
    // set timeout
    time::timeout(time::Duration::from_secs(5), future)
        .await
        .unwrap_or_else(|e| panic!("connect timeout{:?}", e))
}

impl WsServer {
    pub async fn start(config: Config) {
        let (tx, rx) = mpsc::channel(1024);
        let redis = redis::Client::open(config.redis.url()).expect("redis can't open");
        let hub = Manager::new(tx, redis, get_client(&config).await);
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
        let addr = format!("{}:{}", config.websocket.host, config.websocket.port);

        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        tracing::debug!("listening on {}", listener.local_addr().unwrap());
        let mut ws = tokio::spawn(async move {
            println!("start websocket server on {}", addr);
            axum::serve(listener, router).await.unwrap();
        });
        let rpc_addr = format!("{}:{}", config.rpc.ws.host, config.rpc.ws.port);
        let mut rpc = tokio::spawn(async move {
            // start rpc server
            let service = MsgRpcService::new(hub);
            let svc = MsgServiceServer::new(service);
            println!("start rpc server on {}", rpc_addr);
            Server::builder()
                .add_service(svc)
                .serve(rpc_addr.parse().unwrap())
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
                    error!("心跳发送失败：{:?}", e);
                    // break this task, it will end this conn
                    break;
                } else {
                    // tracing::debug!("心跳发送成功");
                }
                tokio::time::sleep(Duration::from_secs(HEART_BEAT_INTERVAL)).await;
            }
        });

        // spawn a new task to receive message
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