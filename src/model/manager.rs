use std::collections::HashMap;
use std::sync::Arc;
use axum::extract::ws::Message;
use futures::SinkExt;
use tokio::sync::{mpsc, RwLock};
use tokio::sync::mpsc::error::SendError;
use crate::model::{ClientSender, Hub, msg::{MessageType, Msg}};

#[derive(Clone)]
pub struct Manager {
    tx: mpsc::Sender<Msg>,
    pub hub: Hub,
}

impl Manager {
    pub fn new(tx: mpsc::Sender<Msg>) -> Self {
        Manager {
            tx,
            hub: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // 注册客户端
    pub async fn register(&self, id: i32, sender: ClientSender) {
        self.hub.write().await.insert(id, sender);
    }
    // 删除客户端
    pub async fn unregister(&self, id: i32) {
        self.hub.write().await.remove(&id);
    }
    pub async fn run(&mut self, mut receiver: mpsc::Receiver<Msg>) {
        tracing::info!("manager start");
        // 循环读取消息
        while let Some(msg) = receiver.recv().await {
            let mut guard = self.hub.write().await;
            match msg.msg_type() {
                MessageType::Single => {
                    if let Some(client) = guard.get_mut(&msg.friend_id()) {
                        client.send(Message::Text(serde_json::to_string(&msg).expect("序列化出错"))).await.expect("发送消息错误")
                    };
                }
                MessageType::Group => {
                    // 根据组id， 查询所有组下的客户端id
                }
                MessageType::Default => {
                    for item in guard.iter_mut() {
                        if let Err(e) = item.1.send(Message::Text(serde_json::to_string(&msg).expect("序列化出错"))).await {
                            // 不能直接删除客户端，会有所有权问题，因为已经借用为mut了
                            tracing::error!("msg send error: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    pub async fn broadcast(&self, msg: Msg) -> Result<(), SendError<Msg>> {
        self.tx.send(msg).await
    }
}
