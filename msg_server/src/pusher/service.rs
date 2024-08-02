use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use abi::errors::Error;
use async_trait::async_trait;
use dashmap::DashMap;
use synapse::service::client::ServiceClient;
use synapse::service::Service;
use tokio::sync::mpsc;
use tonic::transport::{Channel, Endpoint};
use tower::discover::Change;
use tracing::{debug, error};

use abi::config::Config;
use abi::message::msg_service_client::MsgServiceClient;
use abi::message::{GroupMemSeq, Msg, SendGroupMsgRequest, SendMsgRequest};

use super::Pusher;

#[derive(Debug)]
pub struct PusherService {
    ws_rpc_list: Arc<DashMap<SocketAddr, MsgServiceClient<Channel>>>,
    service_center: ServiceClient,
    sub_svr_name: String,
}

impl PusherService {
    pub async fn new(config: &Config) -> Self {
        let sub_svr_name = config.rpc.ws.name.clone();
        let ws_rpc_list = Arc::new(DashMap::new());
        let cloned_list = ws_rpc_list.clone();
        let (tx, mut rx) = mpsc::channel::<Change<SocketAddr, Endpoint>>(100);

        // read the service from the worker
        tokio::spawn(async move {
            while let Some(change) = rx.recv().await {
                debug!("receive service change: {:?}", change);
                match change {
                    Change::Insert(service_id, client) => {
                        match MsgServiceClient::connect(client).await {
                            Ok(client) => {
                                cloned_list.insert(service_id, client);
                            }
                            Err(err) => {
                                error!("connect to ws service error: {:?}", err);
                            }
                        };
                    }
                    Change::Remove(service_id) => {
                        cloned_list.remove(&service_id);
                    }
                }
            }
        });

        utils::get_chan_(config, sub_svr_name.clone(), tx)
            .await
            .unwrap();

        let service_center = ServiceClient::builder()
            .server_host(config.service_center.host.clone())
            .server_port(config.service_center.port)
            .connect_timeout(Duration::from_millis(config.service_center.timeout))
            .build()
            .await
            .unwrap();
        Self {
            ws_rpc_list,
            service_center,
            sub_svr_name,
        }
    }

    pub async fn handle_sub_services(&self, services: Vec<Service>) {
        for service in services {
            let addr = format!("{}:{}", service.address, service.port);
            let socket: SocketAddr = match addr.parse() {
                Ok(sa) => sa,
                Err(err) => {
                    error!("parse socket address error: {:?}", err);
                    continue;
                }
            };
            let addr = format!("{}://{}", service.scheme, addr);
            // connect to ws service
            let endpoint = match Endpoint::from_shared(addr) {
                Ok(ep) => ep.connect_timeout(Duration::from_secs(5)),
                Err(err) => {
                    error!("connect to ws service error: {:?}", err);
                    continue;
                }
            };
            let ws = match MsgServiceClient::connect(endpoint).await {
                Ok(client) => client,
                Err(err) => {
                    error!("connect to ws service error: {:?}", err);
                    continue;
                }
            };
            self.ws_rpc_list.insert(socket, ws);
        }
    }
}

#[async_trait]
impl Pusher for PusherService {
    async fn push_single_msg(&self, request: Msg) -> Result<(), Error> {
        debug!("push msg request: {:?}", request);

        let ws_rpc = self.ws_rpc_list.clone();
        if ws_rpc.is_empty() {
            let mut client = self.service_center.clone();
            let list = client
                .query_with_name(self.sub_svr_name.clone())
                .await
                .map_err(|e| Error::internal_with_details(e.to_string()))?;
            self.handle_sub_services(list).await;
        }

        let request = SendMsgRequest {
            message: Some(request),
        };
        let (tx, mut rx) = mpsc::channel(ws_rpc.len());

        // send message to ws with asynchronous way
        for v in ws_rpc.iter() {
            let tx = tx.clone();
            let service_id = *v.key();
            let mut v = v.clone();
            let request = request.clone();
            tokio::spawn(async move {
                if let Err(err) = v.send_msg_to_user(request).await {
                    tx.send((service_id, err)).await.unwrap();
                };
            });
        }

        // close tx
        drop(tx);

        // todo need to update client list; and need to handle error
        while let Some((service_id, err)) = rx.recv().await {
            ws_rpc.remove(&service_id);
            error!("push msg to {} failed: {}", service_id, err);
        }
        Ok(())
    }

    async fn push_group_msg(&self, msg: Msg, members: Vec<GroupMemSeq>) -> Result<(), Error> {
        debug!("push group msg request: {:?}, {:?}", msg, members);
        // extract request
        let ws_rpc = self.ws_rpc_list.clone();
        if ws_rpc.is_empty() {
            let mut client = self.service_center.clone();
            let list = client
                .query_with_name(self.sub_svr_name.clone())
                .await
                .map_err(|e| Error::internal_with_details(e.to_string()))?;
            self.handle_sub_services(list).await;
        }

        let request = SendGroupMsgRequest {
            message: Some(msg),
            members,
        };
        let (tx, mut rx) = mpsc::channel(ws_rpc.len());
        // send message to ws with asynchronous way
        for v in ws_rpc.iter() {
            let tx = tx.clone();
            let service_id = *v.key();
            let mut v = v.clone();
            let request = request.clone();
            tokio::spawn(async move {
                match v.send_group_msg_to_user(request).await {
                    Ok(_) => {
                        tx.send(Ok(())).await.unwrap();
                    }
                    Err(err) => {
                        tx.send(Err((service_id, err))).await.unwrap();
                    }
                };
            });
        }
        // close tx
        drop(tx);
        // todo need to update client list
        while let Some(Err((service_id, err))) = rx.recv().await {
            ws_rpc.remove(&service_id);
            error!("push msg to {} failed: {}", service_id, err);
        }
        Ok(())
    }
}
