use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tonic::server::NamedService;
use tonic::transport::{Channel, Server};
use tonic::{async_trait, Request, Response, Status};
use tracing::{debug, error, info};

use abi::config::Config;
use abi::errors::Error;
use abi::message::msg_service_client::MsgServiceClient;
use abi::message::push_service_server::{PushService, PushServiceServer};
use abi::message::{SendGroupMsgRequest, SendMsgRequest, SendMsgResponse};
use utils::typos::{GrpcHealthCheck, Registration};

pub struct PusherRpcService {
    ws_rpc: MsgServiceClient<Channel>,
    ws_rpc_list: Arc<DashMap<String, MsgServiceClient<Channel>>>,
}

impl PusherRpcService {
    pub async fn new(config: &Config) -> Self {
        let ws_rpc = Self::get_ws_rpc_client(config).await.unwrap();
        let ws_rpc_list = Arc::new(Self::get_ws_rpc_client_list(config).await.unwrap());
        Self {
            ws_rpc,
            ws_rpc_list,
        }
    }

    pub async fn start(config: &Config) -> Result<(), Error> {
        // register service
        Self::register_service(config).await?;
        info!("<pusher> rpc service register to service register center");

        // for health check
        let (mut reporter, health_service) = tonic_health::server::health_reporter();
        reporter
            .set_serving::<PushServiceServer<PusherRpcService>>()
            .await;
        info!("<pusher> rpc service health check started");

        let pusher_rpc = Self::new(config).await;
        let service = PushServiceServer::new(pusher_rpc);
        info!(
            "<pusher> rpc service started at {}",
            config.rpc.chat.rpc_server_url()
        );

        Server::builder()
            .add_service(health_service)
            .add_service(service)
            .serve(config.rpc.pusher.rpc_server_url().parse().unwrap())
            .await
            .unwrap();
        Ok(())
    }

    async fn get_ws_rpc_client(config: &Config) -> Result<MsgServiceClient<Channel>, Error> {
        // use service register center to get ws rpc url
        let channel =
            utils::get_rpc_channel_by_name(config, &config.rpc.ws.name, &config.rpc.ws.protocol)
                .await?;
        let ws_rpc = MsgServiceClient::new(channel);
        Ok(ws_rpc)
    }

    async fn get_ws_rpc_client_list(
        config: &Config,
    ) -> Result<DashMap<String, MsgServiceClient<Channel>>, Error> {
        // use service register center to get ws rpc url
        let mut ws_list = utils::service_register_center(config)
            .filter_by_name(&config.rpc.ws.name)
            .await?;

        // retry 5 times if no ws rpc url
        if ws_list.is_empty() {
            for i in 0..5 {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                ws_list = utils::service_register_center(config)
                    .filter_by_name(&config.rpc.ws.name)
                    .await?;
                if !ws_list.is_empty() {
                    break;
                }
                if i == 5 {
                    return Err(Error::ServiceNotFound(String::from(&config.rpc.ws.name)));
                }
            }
        }

        let map: DashMap<String, MsgServiceClient<Channel>> = DashMap::with_capacity(ws_list.len());
        for (k, v) in ws_list.into_iter() {
            let url = format!("{}://{}:{}", &config.rpc.ws.protocol, v.address, v.port);
            let client = MsgServiceClient::connect(url)
                .await
                .map_err(|e| Error::TonicError(e.to_string()))?;
            map.insert(k, client);
        }
        Ok(map)
    }

    async fn register_service(config: &Config) -> Result<(), Error> {
        // register service to service register center
        let center = utils::service_register_center(config);
        let grpc = format!(
            "{}/{}",
            config.rpc.pusher.rpc_server_url(),
            <PushServiceServer<PusherRpcService> as NamedService>::NAME
        );
        let check = GrpcHealthCheck {
            name: config.rpc.pusher.name.clone(),
            grpc,
            grpc_use_tls: config.rpc.pusher.grpc_health_check.grpc_use_tls,
            interval: format!("{}s", config.rpc.pusher.grpc_health_check.interval),
        };
        let registration = Registration {
            id: format!("{}-{}", utils::get_host_name()?, &config.rpc.pusher.name),
            name: config.rpc.pusher.name.clone(),
            address: config.rpc.pusher.host.clone(),
            port: config.rpc.pusher.port,
            tags: config.rpc.pusher.tags.clone(),
            check: Some(check),
        };
        center.register(registration).await?;
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
            let service_id = v.key().clone();
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
            let service_id = v.key().clone();
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
        let mut ws_rpc = self.ws_rpc.clone();
        ws_rpc.send_msg_to_user(request).await
    }
}
