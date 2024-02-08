use axum::extract::ws::Message;
use deadpool_diesel::postgres::Pool;
use futures::SinkExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{mpsc, RwLock};

use crate::domain::model::{msg::Msg, Client, Hub};
use crate::infra::repositories::friendship_repo;
use crate::infra::repositories::friendship_repo::create_friend_ship;
use crate::infra::repositories::messages::{insert_msg, msg_delivered, msg_read, NewMsgDb};

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

    pub async fn send_msg(&self, obj_id: &str, msg: &Msg) {
        let mut guard = self.hub.write().await;

        if let Some(clients) = guard.get_mut(obj_id) {
            let content = serde_json::to_string(&msg).expect("序列化出错");
            for (_, client) in clients {
                if let Err(e) = client
                    .sender
                    .write()
                    .await
                    .send(Message::Text(content.clone()))
                    .await
                {
                    // 不能直接删除客户端，会有所有权问题，因为已经借用为mut了
                    tracing::error!("msg send error: {:?}", e);
                } else {
                    // tracing::debug!("消息发送成功--{:?}", client.id.clone());
                }
            }
        }
    }
    // 注册客户端
    pub async fn register(&self, id: String, client: Client) {
        // self.hub.write().await.insert(id, sender);
        let mut guard = self.hub.write().await;
        if let Some(cli) = guard.get_mut(&id) {
            cli.insert(client.id.clone(), client);
        } else {
            let mut hash_map = HashMap::new();
            hash_map.insert(client.id.clone(), client);
            guard.insert(id, hash_map);
        }
    }
    // 删除客户端
    pub async fn unregister(&self, id: String, printer_id: String) {
        // self.hub.write().await.remove(&id);
        let mut guard = self.hub.write().await;
        if let Some(clients) = guard.get_mut(&id) {
            if clients.len() == 1 {
                guard.remove(&id);
            } else {
                clients.remove(&printer_id);
            }
        }
    }
    pub async fn run(&mut self, mut receiver: mpsc::Receiver<Msg>, pool: Pool) {
        tracing::info!("manager start");
        // 循环读取消息
        while let Some(message) = receiver.recv().await {
            match message.clone() {
                Msg::Single(msg) => {
                    tracing::info!("received message: {:?}", &msg);
                    // 数据入库
                    if let Err(err) = insert_msg(&pool, NewMsgDb::from(msg.clone())).await {
                        tracing::error!("消息入库错误！！{:?}", err);
                        continue;
                    }
                    // 入库成功后给客户端回复消息已送达的通知
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::Group(_msg) => {
                    // 根据组id， 查询所有组下的客户端id
                }
                Msg::SingleDeliveredNotice(msg) => {
                    //  消息已送达，更新数据库
                    if let Err(err) = msg_delivered(&pool, vec![msg.msg_id]).await {
                        tracing::error!("更新送达状态错误: {:?}", err);
                    }
                }
                Msg::ReadNotice(msg) => {
                    tracing::info!("received read notice msg: {:?}", &msg);
                    // 更新数据库
                    if let Err(err) = msg_read(&pool, msg.msg_ids).await {
                        tracing::error!("更新已读状态错误: {:?}", err);
                    }
                }
                Msg::SendRelationshipReq(msg) => {
                    tracing::info!("received message: {:?}", msg.clone());
                    // 数据入库
                    let friend_id = msg.friend_id.clone();
                    let res = match create_friend_ship(&pool, msg).await {
                        Err(err) => {
                            tracing::error!("消息入库错误！！{:?}", err);
                            continue;
                        }
                        Ok(res) => res,
                    };
                    // 入库成功后给客户端回复消息已送达的通知
                    // Fixme 可能存在bug
                    let res = Msg::RecRelationship(res);
                    self.send_msg(&friend_id, &res).await;
                }
                Msg::OfflineSync(_) => {}
                Msg::RecRelationship(_) => {}
                Msg::SingleCallOffer(msg) => {
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::SingleCallAgree(msg) => {
                    tracing::info!("received agree: {:?}", &msg);
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::NewIceCandidate(msg) => {
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::SingleCallInvite(msg) => {
                    tracing::info!("received video invite msg: {:?}", &msg);
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::SingleCallInviteAnswer(msg) => {
                    tracing::info!("received answer message: {:?}", &msg);
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::SingleCallInviteCancel(msg) => {
                    // todo 入库
                    if let Err(err) = insert_msg(&pool, NewMsgDb::from(msg.clone())).await {
                        tracing::error!("消息入库错误！！{:?}", err);
                        continue;
                    }
                    tracing::info!("received cancel message: {:?}", &msg);

                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::SingleCallHangUp(msg) => {
                    tracing::info!("received hangup: {:?}", &msg);
                    // todo 入库
                    if let Err(err) = insert_msg(&pool, NewMsgDb::from(msg.clone())).await {
                        tracing::error!("消息入库错误！！{:?}", err);
                        continue;
                    }
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::SingleCallNotAnswer(msg) => {
                    tracing::info!("received not answer message: {:?}", &msg);
                    if let Err(err) = insert_msg(&pool, NewMsgDb::from(msg.clone())).await {
                        tracing::error!("消息入库错误！！{:?}", err);
                        continue;
                    }
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::FriendshipDeliveredNotice(msg) => {
                    //  消息已送达，更新数据库
                    if let Err(err) = friendship_repo::msg_delivered(&pool, vec![msg.msg_id]).await
                    {
                        tracing::error!("更新送达状态错误: {:?}", err);
                    }
                }
            }
        }
    }

    pub async fn broadcast(&self, msg: Msg) -> Result<(), SendError<Msg>> {
        self.tx.send(msg).await
    }
}
