use std::borrow::Cow;
use std::collections::HashMap;
use std::net::IpAddr;
use std::net::SocketAddr;
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
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use futures::{SinkExt, StreamExt};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tonic::transport::Channel;
use tracing::{debug, error, info, warn};

use abi::config::Config;
use abi::errors::Error;
use abi::message::PlatformType;
use synapse::service::{Scheme, ServiceInstance, ServiceRegistryClient};

use crate::client::Client;
use crate::manager::Manager;
use crate::rpc::MsgRpcService;

pub const HEART_BEAT_INTERVAL: u64 = 30;

// 关闭代码
pub const KNOCK_OFF_CODE: u16 = 4001;
pub const UNAUTHORIZED_CODE: u16 = 4002;
pub const MESSAGE_TOO_LARGE_CODE: u16 = 4003;
pub const RATE_LIMITED_CODE: u16 = 4004;
pub const INVALID_MESSAGE_FORMAT_CODE: u16 = 4005;
pub const IDLE_TIMEOUT_CODE: u16 = 4006;

// 连接配置
const MAX_CONN_PER_MINUTE: u32 = 10; // 每分钟最大连接数
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const MAX_MESSAGE_SIZE: usize = 1_048_576; // 1 MB
const MAX_IDLE_TIME: Duration = Duration::from_secs(300); // 5分钟空闲超时

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    New,            // 新建立的连接
    Authenticating, // 正在认证
    Active,         // 活跃状态
    Idle,           // 空闲状态
    Closing,        // 正在关闭
    Closed,         // 已关闭
}

/// 任务退出原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskExitReason {
    PingFailed,         // Ping任务失败
    KnockOff,           // 被踢下线
    ClientDisconnected, // 客户端断开连接
    IdleTimeout,        // 空闲超时
}

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

/// 连接任务管理器
pub struct ConnectionTasks {
    ping_task: JoinHandle<()>,
    watch_task: JoinHandle<()>,
    receive_task: JoinHandle<()>,
    #[allow(dead_code)]
    /// we actually used it
    last_activity: Arc<Mutex<Instant>>,
    state: Arc<Mutex<ConnectionState>>,
}

impl ConnectionTasks {
    /// 创建新的任务集合
    pub fn new(
        device_id: String,
        shared_tx: Arc<RwLock<SplitSink<WebSocket, Message>>>,
        notify_receiver: mpsc::Receiver<()>,
        ws_rx: SplitStream<WebSocket>,
        hub: Manager,
    ) -> Self {
        let last_activity = Arc::new(Mutex::new(Instant::now()));
        let state = Arc::new(Mutex::new(ConnectionState::Active));

        // 创建各个任务
        let ping_task = Self::create_ping_task(
            device_id.clone(),
            shared_tx.clone(),
            last_activity.clone(),
            state.clone(),
        );

        let watch_task = Self::create_watch_task(
            device_id.clone(),
            shared_tx.clone(),
            notify_receiver,
            state.clone(),
        );

        let receive_task = Self::create_receive_task(
            shared_tx.clone(),
            ws_rx,
            last_activity.clone(),
            state.clone(),
            hub,
        );

        Self {
            ping_task,
            watch_task,
            receive_task,
            last_activity,
            state,
        }
    }

    /// 创建Ping任务
    fn create_ping_task(
        device_id: String,
        shared_tx: Arc<RwLock<SplitSink<WebSocket, Message>>>,
        last_activity: Arc<Mutex<Instant>>,
        state: Arc<Mutex<ConnectionState>>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let now = Instant::now();
                let is_idle = {
                    let last = *last_activity.lock().unwrap();
                    now.duration_since(last) > MAX_IDLE_TIME
                };

                if is_idle {
                    info!("Connection idle timeout for device: {}", device_id);
                    *state.lock().unwrap() = ConnectionState::Idle;

                    if let Err(e) = shared_tx
                        .write()
                        .await
                        .send(Message::Close(Some(CloseFrame {
                            code: IDLE_TIMEOUT_CODE,
                            reason: Cow::Owned("Idle timeout".to_string()),
                        })))
                        .await
                    {
                        error!("Failed to send idle timeout close: {}", e);
                    }
                    break;
                }

                if let Err(e) = shared_tx
                    .write()
                    .await
                    .send(Message::Ping(Vec::new()))
                    .await
                {
                    error!("Failed to send ping: {}", e);
                    break;
                }

                tokio::time::sleep(Duration::from_secs(HEART_BEAT_INTERVAL)).await;
            }
        })
    }

    /// 创建Watch任务
    fn create_watch_task(
        device_id: String,
        shared_tx: Arc<RwLock<SplitSink<WebSocket, Message>>>,
        mut notify_receiver: mpsc::Receiver<()>,
        state: Arc<Mutex<ConnectionState>>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            if notify_receiver.recv().await.is_none() {
                info!("Client {} knocked off", device_id);
                *state.lock().unwrap() = ConnectionState::Closing;

                if let Err(e) = shared_tx
                    .write()
                    .await
                    .send(Message::Close(Some(CloseFrame {
                        code: KNOCK_OFF_CODE,
                        reason: Cow::Owned("knock off".to_string()),
                    })))
                    .await
                {
                    error!("Failed to send knock off signal: {}", e);
                }
            }
        })
    }

    /// 创建接收消息任务
    fn create_receive_task(
        shared_tx: Arc<RwLock<SplitSink<WebSocket, Message>>>,
        mut ws_rx: SplitStream<WebSocket>,
        last_activity: Arc<Mutex<Instant>>,
        state: Arc<Mutex<ConnectionState>>,
        hub: Manager,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_rx.next().await {
                // 更新活动时间
                {
                    let mut last = last_activity.lock().unwrap();
                    *last = Instant::now();
                }

                {
                    let mut current_state = state.lock().unwrap();
                    if *current_state == ConnectionState::Idle {
                        *current_state = ConnectionState::Active;
                    }
                }

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
                            continue;
                        }

                        match serde_json::from_str(&text) {
                            Ok(msg) => {
                                if hub.broadcast(msg).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("deserialize error: {:?}, msg: {}", e, text);
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
                        // 收到Pong，更新活动时间即可
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
                            continue;
                        }

                        match bincode::deserialize(&b) {
                            Ok(msg) => {
                                if hub.broadcast(msg).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
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
                                continue;
                            }
                        }
                    }
                }
            }
        })
    }

    /// 等待任务完成并返回退出原因
    pub async fn wait(&mut self) -> TaskExitReason {
        tokio::select! {
            _ = &mut self.ping_task => {
                self.abort_others();

                let state = *self.state.lock().unwrap();
                if state == ConnectionState::Idle {
                    TaskExitReason::IdleTimeout
                } else {
                    TaskExitReason::PingFailed
                }
            }
            _ = &mut self.watch_task => {
                self.abort_others();
                TaskExitReason::KnockOff
            }
            _ = &mut self.receive_task => {
                self.abort_others();
                TaskExitReason::ClientDisconnected
            }
        }
    }

    /// 中止所有任务
    fn abort_others(&self) {
        self.ping_task.abort();
        self.watch_task.abort();
        self.receive_task.abort();
    }

    /// 获取当前连接状态
    pub fn get_state(&self) -> ConnectionState {
        *self.state.lock().unwrap()
    }
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

        let service = router.into_make_service_with_connect_info::<SocketAddr>();
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        let mut ws = tokio::spawn(async move {
            info!("start websocket server on {}", addr);
            axum::serve(listener, service).await.unwrap();
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
        client_ip: axum::extract::ConnectInfo<std::net::SocketAddr>,
        State(state): State<AppState>,
        ws: WebSocketUpgrade,
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
        info!(
            user_id = %user_id,
            device_id = %device_id,
            platform = ?platform,
            "New connection established"
        );

        let (mut ws_tx, ws_rx) = ws.split();

        // 验证token
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
        let (notify_sender, notify_receiver) = tokio::sync::mpsc::channel(1);
        let mut hub = app_state.manager.clone();

        // 创建客户端并注册
        let client = Client {
            user_id: user_id.clone(),
            platform_id: device_id.clone(),
            sender: shared_tx.clone(),
            platform,
            notify_sender,
        };
        hub.register(user_id.clone(), client).await;

        // 创建任务管理器
        let mut tasks = ConnectionTasks::new(
            device_id.clone(),
            shared_tx.clone(),
            notify_receiver,
            ws_rx,
            hub.clone(),
        );

        // 等待任务完成
        let exit_reason = tasks.wait().await;

        // 根据退出原因决定是否需要解注册
        let need_unregister = match exit_reason {
            TaskExitReason::KnockOff => false, // 被踢下线不需要解注册，新客户端已经替换旧的
            _ => true,                         // 其他情况需要解注册
        };

        // 记录连接关闭
        info!(
            user_id = %user_id,
            device_id = %device_id,
            platform = ?platform,
            exit_reason = ?exit_reason,
            need_unregister = need_unregister,
            "Connection closed"
        );

        // 如果需要，从hub中解注册客户端
        if need_unregister {
            hub.unregister(user_id, platform).await;
        }

        debug!("client thread exit {}", hub.hub.iter().count());
    }
}
