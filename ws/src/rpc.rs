use std::result::Result;

use abi::config::Config;
use abi::errors::Error;
use abi::message::msg_service_server::MsgServiceServer;
use abi::message::{msg_service_server::MsgService, SendMsgRequest, SendMsgResponse};
use tonic::async_trait;
use tonic::server::NamedService;
use tonic::transport::Server;
use tracing::{debug, info};
use utils::typos::{GrpcHealthCheck, Registration};

use crate::manager::Manager;

pub struct MsgRpcService {
    manager: Manager,
}

impl MsgRpcService {
    pub fn new(manager: Manager) -> Self {
        Self { manager }
    }

    pub async fn start(manager: Manager, config: &Config) -> Result<(), Error> {
        let service = Self::new(manager);

        // register service to service register center
        service.register_service(config).await?;

        // register the service
        service.register_service(config).await.unwrap();
        info!("<ws> rpc service register to service register center");

        // open health check
        let (mut reporter, health_service) = tonic_health::server::health_reporter();
        reporter
            .set_serving::<MsgServiceServer<MsgRpcService>>()
            .await;
        info!("<ws> rpc service health check started");

        let svc = MsgServiceServer::new(service);
        info!(
            "<ws> rpc service started at {}",
            config.rpc.ws.rpc_server_url()
        );

        Server::builder()
            .add_service(health_service)
            .add_service(svc)
            .serve(config.rpc.ws.rpc_server_url().parse().unwrap())
            .await
            .unwrap();
        Ok(())
    }

    async fn register_service(&self, config: &Config) -> Result<(), Error> {
        // register service to service register center
        let center = utils::service_register_center(config);
        let grpc = format!(
            "{}/{}",
            config.rpc.ws.rpc_server_url(),
            <MsgServiceServer<MsgRpcService> as NamedService>::NAME
        );
        let check = GrpcHealthCheck {
            name: config.rpc.ws.name.clone(),
            grpc,
            grpc_use_tls: config.rpc.ws.grpc_health_check.grpc_use_tls,
            interval: format!("{}s", config.rpc.ws.grpc_health_check.interval),
        };
        let registration = Registration {
            id: "xmj-ws".to_string(),
            name: config.rpc.ws.name.clone(),
            address: config.rpc.ws.host.clone(),
            port: config.rpc.ws.port,
            tags: config.rpc.ws.tags.clone(),
            check: Some(check),
        };
        center.register(registration).await?;
        Ok(())
    }
}

#[async_trait]
impl MsgService for MsgRpcService {
    async fn send_message(
        &self,
        request: tonic::Request<SendMsgRequest>,
    ) -> Result<tonic::Response<SendMsgResponse>, tonic::Status> {
        println!("Got a request: {:?}", request);
        let msg = request.into_inner().message;
        if msg.is_none() {
            return Err(tonic::Status::invalid_argument("message is empty"));
        }
        let msg = msg.unwrap();
        if msg.data.is_none() {
            return Err(tonic::Status::invalid_argument("message is empty"));
        }
        self.manager.broadcast(msg).await?;
        let response = tonic::Response::new(SendMsgResponse {});
        Ok(response)
    }

    /// Send message to user
    /// pusher will procedure this to send message to user
    async fn send_msg_to_user(
        &self,
        request: tonic::Request<SendMsgRequest>,
    ) -> Result<tonic::Response<SendMsgResponse>, tonic::Status> {
        let msg = request.into_inner().message;
        if msg.is_none() {
            return Err(tonic::Status::invalid_argument("message is empty"));
        }
        let msg = msg.unwrap();
        if msg.data.is_none() {
            return Err(tonic::Status::invalid_argument("message is empty"));
        }
        debug!("send message to user: {:?}", msg);
        self.manager.send_msg(msg).await;
        let response = tonic::Response::new(SendMsgResponse {});
        Ok(response)
    }
}
