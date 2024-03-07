use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) mod friend_request_status;
pub(crate) mod friends;
pub(crate) mod group;
pub(crate) mod manager;
pub(crate) mod msg;
pub(crate) mod user;

type ClientSender = Arc<RwLock<SplitSink<WebSocket, Message>>>;

// 修改客户端为数组，包含桌面端以及移动端。。。Hub的key就是用户id，这样在发送消息时就不需要遍历查找了，提高一点性能
pub struct Client {
    pub id: String,
    pub sender: ClientSender,
}

type Hub = Arc<RwLock<HashMap<String, HashMap<String, Client>>>>;
