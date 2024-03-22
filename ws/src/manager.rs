use crate::client::Client;
use abi::errors::Error;
use abi::msg::msg_wrapper::Msg;
use dashmap::DashMap;
use kafka::producer::Producer;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::error;

type UserID = String;
type PlatformID = String;
/// client hub
type Hub = Arc<DashMap<UserID, DashMap<PlatformID, Client>>>;

/// manage the client
#[derive(Clone)]
pub struct Manager {
    tx: mpsc::Sender<Msg>,
    pub hub: Hub,
    pub redis: redis::Client,
    pub kafka: Arc<Producer>,
}

#[allow(dead_code)]
impl Manager {
    pub fn new(tx: mpsc::Sender<Msg>, redis: redis::Client, kafka: Producer) -> Self {
        Manager {
            tx,
            hub: Arc::new(DashMap::new()),
            redis,
            kafka: Arc::new(kafka),
        }
    }
    pub async fn send_group(&self, obj_ids: &Vec<String>, msg: &Msg) {
        for id in obj_ids {
            if let Some(clients) = self.hub.get(id) {
                self.send_msg_to_clients(&clients, msg).await;
            }
        }
    }

    pub async fn send_single_msg(&self, obj_id: &str, msg: &Msg) {
        if let Some(clients) = self.hub.get(obj_id) {
            self.send_msg_to_clients(&clients, msg).await;
        }
    }

    async fn send_msg_to_clients(&self, clients: &DashMap<PlatformID, Client>, msg: &Msg) {
        for client in clients.iter() {
            let content = serde_json::to_string(&msg).expect("序列化出错");
            if let Err(e) = client.value().send_text(content).await {
                error!("msg send error: {:?}", e);
            } else {
                // debug!("消息发送成功--{:?}", client.id.clone());
            }
        }
    }
    // 注册客户端
    // todo check platform id, if existed already, kick offline
    pub async fn register(&mut self, id: String, client: Client) {
        if let Some(cli) = self.hub.get_mut(&id) {
            cli.insert(client.platform_id.clone(), client);
        } else {
            let hash_map = DashMap::new();
            hash_map.insert(client.user_id.clone(), client);
            self.hub.insert(id, hash_map);
        }
    }
    // 删除客户端
    pub async fn unregister(&mut self, id: String, printer_id: String) {
        // self.hub.write().await.remove(&id);
        if let Some(clients) = self.hub.get_mut(&id) {
            if clients.len() == 1 {
                self.hub.remove(&id);
            } else {
                clients.remove(&printer_id);
            }
        }
    }

    pub async fn run(&mut self, _receiver: mpsc::Receiver<Msg>) {}
    pub async fn broadcast(&self, msg: Msg) -> Result<(), Error> {
        self.tx.send(msg).await.map_err(|_| Error::BroadCastError)
    }
}
