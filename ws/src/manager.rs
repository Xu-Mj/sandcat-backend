use std::sync::Arc;

use abi::config::Config;
use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::client::Client;
use abi::errors::Error;
use abi::message::chat_service_client::ChatServiceClient;
use abi::message::{ContentType, Msg, MsgResponse, MsgType, PlatformType, SendMsgRequest};
use cache::Cache;
use utils::service_discovery::LbWithServiceDiscovery;

type UserID = String;
/// client hub
type Hub = Arc<DashMap<UserID, DashMap<PlatformType, Client>>>;

/// manage the client
#[derive(Clone)]
pub struct Manager {
    tx: mpsc::Sender<Msg>,
    pub hub: Hub,
    pub cache: Arc<dyn Cache>,
    pub chat_rpc: ChatServiceClient<LbWithServiceDiscovery>,
}

#[allow(dead_code)]
impl Manager {
    pub async fn new(tx: mpsc::Sender<Msg>, config: &Config) -> Self {
        let cache = cache::cache(config);
        let chat_rpc = utils::get_rpc_client(config, config.rpc.chat.name.clone())
            .await
            .expect("chat rpc can't open");
        Manager {
            tx,
            hub: Arc::new(DashMap::new()),
            cache,
            chat_rpc,
        }
    }

    pub async fn send_group(&self, obj_ids: &Vec<String>, mut msg: Msg) {
        for id in obj_ids {
            if let Some(clients) = self.hub.get(id) {
                // need to query the users seq
                let seq = match self.cache.get_seq(id).await {
                    Ok(seq) => seq,
                    Err(e) => {
                        error!("get seq error: {:?}", e);
                        continue;
                    }
                };
                msg.seq = seq;
                self.send_msg_to_clients(&clients, &msg).await;
            }
        }
    }

    pub async fn send_single_msg(&self, obj_id: &str, msg: &Msg) {
        if let Some(clients) = self.hub.get(obj_id) {
            self.send_msg_to_clients(&clients, msg).await;
        }
    }

    async fn send_msg_to_clients(&self, clients: &DashMap<PlatformType, Client>, msg: &Msg) {
        for client in clients.iter() {
            let content = match bincode::serialize(msg) {
                Ok(res) => res,
                Err(_) => {
                    error!("msg serialize error");
                    return;
                }
            };
            if let Err(e) = client.value().send_binary(content).await {
                error!("msg send error: {:?}", e);
            } else {
                // debug!("message send success--{:?}", client.id.clone());
            }
        }
    }

    // 注册客户端
    pub async fn register(&mut self, id: String, client: Client) {
        self.hub
            .entry(id)
            .or_default()
            .insert(client.platform, client);
    }

    pub async fn unregister(&mut self, id: String, platform: PlatformType) {
        let mut flag = false;
        if let Some(clients) = self.hub.get_mut(&id) {
            if clients.len() == 1 {
                flag = true;
            } else {
                clients.remove(&platform);
            }
        };
        if flag {
            self.hub.remove(&id);
        }
        debug!("unregister client: {:?}", id);
    }

    pub async fn run(&mut self, mut receiver: mpsc::Receiver<Msg>) {
        info!("manager start");
        // 循环读取消息
        while let Some(mut message) = receiver.recv().await {
            // request the message rpc to get server_msg_id
            debug!("receive message: {:?}", message);
            match self
                .chat_rpc
                .send_msg(SendMsgRequest {
                    message: Some(message.clone()),
                })
                .await
            {
                Ok(res) => {
                    // reply send success
                    let response = res.into_inner();
                    if response.err.is_empty() {
                        debug!("send message success");
                    } else {
                        error!("send message error: {:?}", response.err);
                        message.content_type = ContentType::Error as i32;
                    }
                    message.msg_type = MsgType::MsgRecResp as i32;
                    message.server_id.clone_from(&response.server_id);
                    message.content = response.err.into_bytes();
                }
                Err(err) => {
                    error!("send message error: {:?}", err);
                    let response = MsgResponse::from(err);
                    message.content = response.err.into_bytes();
                }
            }

            // reply result to sender
            debug!("reply message:{:?}", message);
            self.send_single_msg(&message.send_id, &message).await;
        }
    }

    pub async fn broadcast(&self, msg: Msg) -> Result<(), Error> {
        self.tx.send(msg).await.map_err(|_| Error::BroadCastError)
    }
}
