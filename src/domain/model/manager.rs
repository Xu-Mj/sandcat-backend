use axum::extract::ws::Message;
use deadpool_diesel::postgres::Pool;
use futures::SinkExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};

use crate::domain::model::{msg::Msg, Client, Hub};
use crate::infra::repositories::friendship_repo;
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
            for client in clients.values_mut() {
                if let Err(e) = client
                    .sender
                    .write()
                    .await
                    .send(Message::Text(content.clone()))
                    .await
                {
                    // 不能直接删除客户端，会有所有权问题，因为已经借用为mut了
                    error!("msg send error: {:?}", e);
                } else {
                    // debug!("消息发送成功--{:?}", client.id.clone());
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
        info!("manager start");
        // 循环读取消息
        while let Some(message) = receiver.recv().await {
            match message.clone() {
                Msg::Single(msg) => {
                    info!("received message: {:?}", &msg);
                    // 数据入库
                    if let Err(err) = insert_msg(&pool, NewMsgDb::from(msg.clone())).await {
                        error!("消息入库错误！！{:?}", err);
                        continue;
                    }
                    // 入库成功后给客户端回复消息已送达的通知
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::Group(msg) => {
                    // 根据组id， 查询所有组下的客户端id
                    debug!("received group message: {:?}", msg);
                }
                Msg::SingleDeliveredNotice(msg) => {
                    //  消息已送达，更新数据库
                    if let Err(err) = msg_delivered(&pool, vec![msg.msg_id]).await {
                        error!("更新送达状态错误: {:?}", err);
                    }
                }
                Msg::ReadNotice(msg) => {
                    info!("received read notice msg: {:?}", &msg);
                    // 更新数据库
                    if let Err(err) = msg_read(&pool, msg.msg_ids).await {
                        error!("更新已读状态错误: {:?}", err);
                    }
                }
                Msg::SendRelationshipReq(_msg) => {
                    // FIXME 目前走的http的方式，存在这巨大的逻辑问题
                    // 1. 好友发送请求
                    // 2. 数据入库，返回FriendshipWithUser
                    // 3.1 被请求方在线，直接将SendRelationshipReq(FriendshipWithUser)消息发送给目标用户，
                    //       服务端不需要处理SendRelationshipReq(FriendshipWithUser)类型，只需要客户端处理
                    // 3.2 被请求方不在线，--等待上线时查询好友请求列表，ws返回SendRelationshipReq消息
                    // 3.3 客户端处理
                    // 4. 被请求方同意好友请求
                    // 4.1 发送http请求调用agree
                    // 5 update数据库，根据friendship数据查询相关用户数据，生成FriendWithUser并返回
                    // 5.1 请求方如果在线，发送RelationshipRes(FriendWithUser)
                    // 5.2 请求方不在线，--等待上线时查询好友请求响应列表，ws返回RelationshipRes消息
                    // 5.3 客户端处理
                    /*  info!("received message: {:?}", msg.clone());
                    // 数据入库
                    let friend_id = msg.friend_id.clone();
                    let res = match create_friend_ship(&pool, msg).await {
                        Err(err) => {
                            error!("消息入库错误！！{:?}", err);
                            continue;
                        }
                        Ok(res) => res,
                    };
                    // 入库成功后给客户端回复消息已送达的通知
                    // Fixme 可能存在bug
                    let res = Msg::RecRelationship(res);
                    self.send_msg(&friend_id, &res).await;*/
                }
                Msg::RelationshipRes(_msg) => {}
                Msg::OfflineSync(_) => {}
                Msg::RecRelationship(_) => {}
                Msg::SingleCallOffer(msg) => {
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::SingleCallAgree(msg) => {
                    info!("received agree: {:?}", &msg);
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::NewIceCandidate(msg) => {
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::SingleCallInvite(msg) => {
                    info!("received video invite msg: {:?}", &msg);
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::SingleCallInviteAnswer(msg) => {
                    info!("received answer message: {:?}", &msg);
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::SingleCallInviteCancel(msg) => {
                    // todo 入库
                    if let Err(err) = insert_msg(&pool, NewMsgDb::from(msg.clone())).await {
                        error!("消息入库错误！！{:?}", err);
                        continue;
                    }
                    info!("received cancel message: {:?}", &msg);

                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::SingleCallHangUp(msg) => {
                    info!("received hangup: {:?}", &msg);
                    // todo 入库
                    if let Err(err) = insert_msg(&pool, NewMsgDb::from(msg.clone())).await {
                        error!("消息入库错误！！{:?}", err);
                        continue;
                    }
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::SingleCallNotAnswer(msg) => {
                    info!("received not answer message: {:?}", &msg);
                    if let Err(err) = insert_msg(&pool, NewMsgDb::from(msg.clone())).await {
                        error!("消息入库错误！！{:?}", err);
                        continue;
                    }
                    self.send_msg(&msg.friend_id, &message).await;
                }
                Msg::FriendshipDeliveredNotice(msg) => {
                    //  消息已送达，更新数据库
                    if let Err(err) = friendship_repo::msg_delivered(&pool, vec![msg.msg_id]).await
                    {
                        error!("更新送达状态错误: {:?}", err);
                    }
                }
            }
        }
    }

    pub async fn broadcast(&self, msg: Msg) -> Result<(), SendError<Msg>> {
        self.tx.send(msg).await
    }
}
