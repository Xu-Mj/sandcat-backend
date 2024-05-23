use abi::message::PlatformType;
use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use futures::SinkExt;
use std::sync::Arc;
use tokio::sync::RwLock;

type ClientSender = Arc<RwLock<SplitSink<WebSocket, Message>>>;

/// client
pub struct Client {
    // hold a ws connection sender
    pub sender: ClientSender,
    // user id
    pub user_id: String,
    // platform id
    pub platform_id: String,
    pub platform: PlatformType,
}

#[allow(dead_code)]
impl Client {
    pub async fn send_text(&self, msg: String) -> Result<(), axum::Error> {
        self.sender.write().await.send(Message::Text(msg)).await
    }

    pub async fn send_binary(&self, msg: Vec<u8>) -> Result<(), axum::Error> {
        self.sender.write().await.send(Message::Binary(msg)).await
    }
}
