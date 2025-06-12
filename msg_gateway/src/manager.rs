use std::sync::Arc;

use abi::config::Config;
use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::client::Client;
use abi::errors::Error;
use abi::message::chat_service_client::ChatServiceClient;
use abi::message::{
    ContentType, GroupMemSeq, Msg, MsgResponse, MsgType, PlatformType, SendMsgRequest,
};
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

    pub async fn send_group(&self, obj_ids: Vec<GroupMemSeq>, mut msg: Msg) {
        self.send_to_self(&msg.sender_id, &msg).await;

        // set send sequence to 0
        msg.send_seq = 0;

        for mem in obj_ids {
            if let Some(clients) = self.hub.get(&mem.mem_id) {
                // Modify only the seq in the message and serialize it.
                msg.seq = mem.cur_seq;

                // Send message to all clients
                self.send_msg_to_clients(&clients, &msg).await;
            }
        }
    }

    async fn send_to_self(&self, id: &str, msg: &Msg) {
        if let Some(client) = self.hub.get(id) {
            // send to self which another platform client
            let platform = if msg.platform == PlatformType::Mobile as i32 {
                PlatformType::Desktop
            } else {
                PlatformType::Mobile
            };
            if let Some(sender) = client.get(&platform) {
                let content = match bincode::serialize(msg) {
                    Ok(res) => res,
                    Err(_) => {
                        error!("msg serialize error");
                        return;
                    }
                };
                if let Err(e) = sender.send_binary(content).await {
                    error!("send to self error: {}", e)
                }
            }
        }
    }

    pub async fn send_single_msg(&self, obj_id: &str, msg: &Msg) {
        if let Some(clients) = self.hub.get(obj_id) {
            self.send_msg_to_clients(&clients, msg).await;
        }
        self.send_to_self(&msg.sender_id, msg).await;
    }

    async fn send_msg_to_clients(&self, clients: &DashMap<PlatformType, Client>, msg: &Msg) {
        match clients.len() {
            0 => error!("no client found"),
            1 => {
                let content = match bincode::serialize(msg) {
                    Ok(res) => res,
                    Err(e) => {
                        error!("msg serialize error: {}", e);
                        return;
                    }
                };
                if let Some(client) = clients.iter().next() {
                    if let Err(e) = client.value().send_binary(content).await {
                        error!("send message error: {}", e);
                    }
                }
            }
            2 => {
                let content = match bincode::serialize(msg) {
                    Ok(res) => res,
                    Err(e) => {
                        error!("msg serialize error: {}", e);
                        return;
                    }
                };
                let mut iter = clients.iter();
                if let Some(first_client) = iter.next() {
                    if let Err(e) = first_client.value().send_binary(content.clone()).await {
                        error!("send message error: {}", e);
                    }
                }
                if let Some(second_client) = iter.next() {
                    if let Err(e) = second_client.value().send_binary(content).await {
                        error!("send message error: {}", e);
                    }
                }
            }
            _ => warn!("Unexpected number of clients: {}", clients.len()),
        }
    }

    // register client
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

        // read the message from the channel
        while let Some(mut message) = receiver.recv().await {
            self.process_message(&mut message).await;

            // reply send result
            debug!("reply message:{:?}", message);
            self.send_single_msg(&message.sender_id, &message).await;
        }
    }

    async fn process_message(&mut self, message: &mut Msg) {
        // increment the send sequence in cache
        //  we do not operate the database here about saving send sequence
        // we do that in the consumer module
        // even if the increment fails, it is not a problem
        match self.cache.incr_send_seq(&message.sender_id).await {
            Ok((seq, _, _)) => message.send_seq = seq,
            Err(e) => {
                self.create_error_message(message, e);
                return;
            }
        }

        // send message through gRPC
        match self.send_rpc_message(message.clone()).await {
            Ok(response) => {
                if response.err.is_empty() {
                    debug!("send message success");
                    message.content.clear();
                } else {
                    error!("send message error: {:?}", response.err);
                    self.create_error_message(message, response.err)
                }
                message.msg_type = MsgType::MsgRecResp as i32;
                message.server_id.clone_from(&response.server_id);
                message.send_time = response.send_time;
            }
            Err(err) => {
                error!("send message error: {:?}", err);

                self.create_error_message(message, err);
            }
        }
    }

    async fn send_rpc_message(&self, message: Msg) -> Result<MsgResponse, tonic::Status> {
        let mut chat_rpc = self.chat_rpc.clone();
        chat_rpc
            .send_msg(SendMsgRequest {
                message: Some(message),
            })
            .await
            .map(|res| res.into_inner())
    }

    fn create_error_message(&self, message: &mut Msg, error: impl ToString) {
        message.content_type = ContentType::Error as i32;
        message.msg_type = MsgType::MsgRecResp as i32;
        message.content = error.to_string().into_bytes();
    }

    pub async fn broadcast(&self, msg: Msg) -> Result<(), Error> {
        self.tx
            .send(msg)
            .await
            .map_err(|e| Error::broadcast(Box::new(e)))
    }
}
