use std::borrow::Cow;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

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
use abi::message::PlatformType;
use synapse::service::{Scheme, ServiceInstance, ServiceRegistryClient};

use crate::client::Client;
use crate::manager::Manager;
use crate::rpc::MsgRpcService;

pub const HEART_BEAT_INTERVAL: u64 = 30;

pub const KNOCK_OFF_CODE: u16 = 4001;
pub const UNAUTHORIZED_CODE: u16 = 4002;
pub const MESSAGE_TOO_LARGE_CODE: u16 = 4003;
pub const RATE_LIMITED_CODE: u16 = 4004;
pub const INVALID_MESSAGE_FORMAT_CODE: u16 = 4005;

const MAX_CONN_PER_MINUTE: u32 = 10; // 每分钟最大连接数
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const MAX_MESSAGE_SIZE: usize = 1_048_576; // 1 MB

#[derive(Clone)]
pub struct AppState {
    manager: Manager,
    jwt_secret: String,
    rate_limiter: Arc<Mutex<HashMap<IpAddr, (u32, Instant)>>>,
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
            rate_limiter: Arc::new(Mutex::new(HashMap::new())),
        };

        // run axum server
        let router = Router::new()
            .route(
                "/ws/:user_id/:device_id/:platform",
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

    fn verify_token(token: String, user_id: &str, jwt_secret: &String) -> Result<Claims, Error> {
        let validation = Validation::default();

        let token_data = match decode::<Claims>(
            &token,
            &DecodingKey::from_secret(jwt_secret.as_bytes()),
            &validation,
        ) {
            Ok(data) => data,
            Err(err) => match *err.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    return Err(Error::unauthorized(err, "Token expired"))
                }
                jsonwebtoken::errors::ErrorKind::InvalidToken => {
                    return Err(Error::unauthorized(err, "Invalid token format"))
                }
                _ => return Err(Error::unauthorized(err, "Token expired")),
            },
        };

        // 验证token中的用户ID与请求的用户ID匹配
        if token_data.claims.sub != user_id {
            return Err(Error::unauthorized_with_details(
                "User ID mismatch in token",
            ));
        }

        Ok(token_data.claims)
    }

    pub async fn websocket_handler(
        Path((user_id, device_id, platform)): Path<(String, String, i32)>,
        headers: axum::http::HeaderMap,
        ws: WebSocketUpgrade,
        client_ip: axum::extract::ConnectInfo<std::net::SocketAddr>,
        State(state): State<AppState>,
    ) -> impl IntoResponse {
        // 获取客户端IP
        let ip = client_ip.0.ip();

        // 检查连接速率
        let rate_limited = {
            let mut rate_limiter = state.rate_limiter.lock().unwrap();
            let now = Instant::now();

            if let Some((count, timestamp)) = rate_limiter.get(&ip).map(|(c, t)| (*c, *t)) {
                if now.duration_since(timestamp) < RATE_LIMIT_WINDOW && count >= MAX_CONN_PER_MINUTE
                {
                    true
                } else if now.duration_since(timestamp) >= RATE_LIMIT_WINDOW {
                    rate_limiter.insert(ip, (1, now));
                    false
                } else {
                    rate_limiter.insert(ip, (count + 1, timestamp));
                    false
                }
            } else {
                // 第一次连接
                rate_limiter.insert(ip, (1, now));
                false
            }
        };

        if rate_limited {
            return (
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                "Connection rate limit exceeded".to_string(),
            )
                .into_response();
        }

        // 从Authorization头获取token
        let token = match headers.get("Authorization") {
            Some(auth_header) => {
                let auth_str = auth_header.to_str().unwrap_or_default();
                // 支持"Bearer {token}"格式
                if let Some(prefix) = auth_str.strip_prefix("Bearer ") {
                    prefix.to_string()
                } else {
                    auth_str.to_string()
                }
            }
            None => {
                return (
                    axum::http::StatusCode::UNAUTHORIZED,
                    "Missing Authorization header".to_string(),
                )
                    .into_response();
            }
        };

        let platform = PlatformType::try_from(platform).unwrap_or_default();
        ws.on_upgrade(move |socket| {
            Self::websocket(user_id, device_id, token, platform, socket, state)
        })
    }

    pub async fn websocket(
        user_id: String,
        device_id: String,
        token: String,
        platform: PlatformType,
        ws: WebSocket,
        app_state: AppState,
    ) {
        tracing::info!(
            "client {} connected, user id : {}",
            user_id.clone(),
            device_id.clone()
        );
        let (mut ws_tx, mut ws_rx) = ws.split();
        // validate token
        if let Err(err) = Self::verify_token(token, &user_id, &app_state.jwt_secret) {
            warn!("verify token error: {:?}", err);
            if let Err(e) = ws_tx
                .send(Message::Close(Some(CloseFrame {
                    code: UNAUTHORIZED_CODE,
                    reason: Cow::Owned(format!("Authentication failed: {}", err)),
                })))
                .await
            {
                error!("send auth error to client failed: {}", e);
            }
            return;
        }
        let shared_tx = Arc::new(RwLock::new(ws_tx));
        let (notify_sender, mut notify_receiver) = tokio::sync::mpsc::channel(1);
        let mut hub = app_state.manager.clone();
        let client = Client {
            user_id: user_id.clone(),
            platform_id: device_id.clone(),
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
            // if the client is dropped, the notify_receiver will be closed
            // it will receive a None value
            if notify_receiver.recv().await.is_none() {
                info!("client {} knock off", device_id);
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
            let closure = async |e: Error| {
                error!("deserialize error: {:?}", e);
                if let Err(e) = shared_tx
                    .write()
                    .await
                    .send(Message::Close(Some(CloseFrame {
                        code: INVALID_MESSAGE_FORMAT_CODE,
                        reason: Cow::Owned("Invalid message format".to_string()),
                    })))
                    .await
                {
                    error!("Failed to send close frame: {}", e);
                }
            };
            while let Some(Ok(msg)) = ws_rx.next().await {
                // 处理消息
                match msg {
                    Message::Text(text) => {
                        if text.len() > MAX_MESSAGE_SIZE {
                            error!("Message too large: {} bytes", text.len());
                            if let Err(e) = shared_tx
                                .write()
                                .await
                                .send(Message::Close(Some(CloseFrame {
                                    code: MESSAGE_TOO_LARGE_CODE,
                                    reason: Cow::Owned("Message too large".to_string()),
                                })))
                                .await
                            {
                                error!("Failed to send close frame: {}", e);
                            }
                            break;
                        }

                        match serde_json::from_str(&text) {
                            Ok(msg) => {
                                if cloned_hub.broadcast(msg).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                closure(e.into()).await;
                                // error!("deserialize error: {:?}; source: {text}", e);
                                // if let Err(e) = shared_tx
                                //     .write()
                                //     .await
                                //     .send(Message::Close(Some(CloseFrame {
                                //         code: INVALID_MESSAGE_FORMAT_CODE,
                                //         reason: Cow::Owned("Invalid message format".to_string()),
                                //     })))
                                //     .await
                                // {
                                //     error!("Failed to send close frame: {}", e);
                                // }
                                continue;
                            }
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
                        if b.len() > MAX_MESSAGE_SIZE {
                            error!("Binary message too large: {} bytes", b.len());
                            if let Err(e) = shared_tx
                                .write()
                                .await
                                .send(Message::Close(Some(CloseFrame {
                                    code: MESSAGE_TOO_LARGE_CODE,
                                    reason: Cow::Owned("Message too large".to_string()),
                                })))
                                .await
                            {
                                error!("Failed to send close frame: {}", e);
                            }
                            break;
                        }

                        match bincode::deserialize(&b) {
                            Ok(msg) => {
                                if cloned_hub.broadcast(msg).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                closure(e.into()).await;
                                // error!("deserialize error: {:?}； source: {:?}", e, b);
                                // if let Err(e) = shared_tx
                                //     .write()
                                //     .await
                                //     .send(Message::Close(Some(CloseFrame {
                                //         code: INVALID_MESSAGE_FORMAT_CODE,
                                //         reason: Cow::Owned("Invalid message format".to_string()),
                                //     })))
                                //     .await
                                // {
                                //     error!("Failed to send close frame: {}", e);
                                // }
                                continue;
                            }
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
