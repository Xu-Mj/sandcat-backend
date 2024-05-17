use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use synapse::health::{HealthCheck, HealthServer, HealthService};
use synapse::service::{Scheme, ServiceInstance, ServiceRegistryClient};
use tokio::sync::mpsc;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::{async_trait, Request, Response, Status};
use tower::discover::Change;
use tracing::{debug, error, info};

use abi::config::Config;
use abi::errors::Error;
use abi::message::msg_service_client::MsgServiceClient;
use abi::message::push_service_server::{PushService, PushServiceServer};
use abi::message::{SendGroupMsgRequest, SendMsgRequest, SendMsgResponse};

pub struct PusherRpcService {
    ws_rpc_list: Arc<DashMap<SocketAddr, MsgServiceClient<Channel>>>,
}

impl PusherRpcService {
    pub async fn new(config: &Config) -> Self {
        let ws_rpc_list = Arc::new(DashMap::new());
        let cloned_list = ws_rpc_list.clone();
        let (tx, mut rx) = mpsc::channel::<Change<SocketAddr, Endpoint>>(100);

        // read the service from the worker
        tokio::spawn(async move {
            while let Some(change) = rx.recv().await {
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

        utils::get_chan_(config, config.rpc.ws.name.clone(), tx)
            .await
            .unwrap();

        Self { ws_rpc_list }
    }

    pub async fn start(config: &Config) {
        // register service
        Self::register_service(config).await.unwrap();
        info!("<pusher> rpc service register to service register center");

        // for health check
        let health_service = HealthServer::new(HealthService::new());
        info!("<pusher> rpc service health check started");

        let pusher_rpc = Self::new(config).await;
        let service = PushServiceServer::new(pusher_rpc);
        info!(
            "<pusher> rpc service started at {}",
            config.rpc.pusher.rpc_server_url()
        );

        Server::builder()
            .add_service(health_service)
            .add_service(service)
            .serve(config.rpc.pusher.rpc_server_url().parse().unwrap())
            .await
            .unwrap();
    }

    async fn register_service(config: &Config) -> Result<(), Error> {
        // register service to service register center
        let addr = format!(
            "{}://{}:{}",
            config.service_center.protocol, config.service_center.host, config.service_center.port
        );
        let endpoint = Endpoint::from_shared(addr)
            .map_err(|e| Error::TonicError(e.to_string()))?
            .connect_timeout(Duration::from_secs(5));
        let mut client = ServiceRegistryClient::connect(endpoint)
            .await
            .map_err(|e| Error::TonicError(e.to_string()))?;
        let service = ServiceInstance {
            id: format!("{}-{}", utils::get_host_name()?, &config.rpc.pusher.name),
            name: config.rpc.pusher.name.clone(),
            address: config.rpc.pusher.host.clone(),
            port: config.rpc.pusher.port as i32,
            tags: config.rpc.pusher.tags.clone(),
            version: "".to_string(),
            metadata: Default::default(),
            health_check: Some(HealthCheck {
                endpoint: "".to_string(),
                interval: 10,
                timeout: 10,
                retries: 10,
                scheme: Scheme::from(config.rpc.db.protocol.as_str()) as i32,
                tls_domain: None,
            }),
            status: 0,
            scheme: Scheme::from(config.rpc.db.protocol.as_str()) as i32,
        };
        client.register_service(service).await.unwrap();
        Ok(())
    }
}

#[async_trait]
impl PushService for PusherRpcService {
    async fn push_single_msg(
        &self,
        request: Request<SendMsgRequest>,
    ) -> Result<Response<SendMsgResponse>, Status> {
        debug!("push msg request: {:?}", request);
        // extract request
        let request = request.into_inner();

        let ws_rpc = self.ws_rpc_list.clone();
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
        Ok(Response::new(SendMsgResponse {}))
    }

    async fn push_group_msg(
        &self,
        request: Request<SendGroupMsgRequest>,
    ) -> Result<Response<SendMsgResponse>, Status> {
        debug!("push group msg request: {:?}", request);
        // extract request
        let request = request.into_inner();
        let ws_rpc = self.ws_rpc_list.clone();
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
        Ok(Response::new(SendMsgResponse {}))
    }

    /// push message to ws, need to think about this
    async fn push_msg(
        &self,
        request: Request<SendMsgRequest>,
    ) -> Result<Response<SendMsgResponse>, Status> {
        // push message to ws
        debug!("push msg request: {:?}", request);
        // let mut ws_rpc = self.ws_rpc.clone();
        // ws_rpc.send_msg_to_user(request).await
        Ok(Response::new(SendMsgResponse {}))
    }
}
