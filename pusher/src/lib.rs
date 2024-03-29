use tonic::server::NamedService;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::{async_trait, Request, Response, Status};
use tracing::{debug, info};

use abi::config::Config;
use abi::errors::Error;
use abi::message::msg_service_client::MsgServiceClient;
use abi::message::push_service_server::{PushService, PushServiceServer};
use abi::message::{SendMsgRequest, SendMsgResponse};
use utils::typos::{GrpcHealthCheck, Registration};

pub struct PusherRpcService {
    ws_rpc: MsgServiceClient<Channel>,
}

impl PusherRpcService {
    pub async fn new(config: &Config) -> Self {
        let ws_rpc = Self::get_ws_rpc_client(config).await.unwrap();
        Self { ws_rpc }
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
        let protocol = config.rpc.ws.protocol.clone();
        let ws_list = utils::get_service_list_by_name(config, &config.rpc.ws.name).await?;
        let endpoints = ws_list.values().map(|v| {
            let url = format!("{}://{}:{}", &protocol, v.address, v.port);
            Endpoint::from_shared(url).unwrap()
        });
        debug!("ws rpc endpoints: {:?}", endpoints);

        let channel = Channel::balance_list(endpoints);
        let ws_rpc = MsgServiceClient::new(channel);
        Ok(ws_rpc)
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
