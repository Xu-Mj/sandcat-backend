use std::result::Result;

use abi::config::Config;
use abi::errors::Error;
use abi::message::msg_service_server::MsgServiceServer;
use abi::message::{
    msg_service_server::MsgService, SendGroupMsgRequest, SendMsgRequest, SendMsgResponse,
};
use tonic::server::NamedService;
use tonic::transport::Server;
use tonic::{async_trait, Request, Response, Status};
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
        // register service to service register center
        Self::register_service(config).await?;
        info!("<ws> rpc service register to service register center");

        // open health check
        let (mut reporter, health_service) = tonic_health::server::health_reporter();
        reporter
            .set_serving::<MsgServiceServer<MsgRpcService>>()
            .await;
        info!("<ws> rpc service health check started");

        let service = Self::new(manager);
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

    async fn register_service(config: &Config) -> Result<(), Error> {
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
            id: format!("{}-{}", utils::get_host_name()?, &config.rpc.ws.name),
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
        request: Request<SendMsgRequest>,
    ) -> Result<Response<SendMsgResponse>, Status> {
        debug!("Got a request: {:?}", request);
        let msg = request
            .into_inner()
            .message
            .ok_or_else(|| Status::invalid_argument("message is empty"))?;
        self.manager.broadcast(msg).await?;
        let response = Response::new(SendMsgResponse {});
        Ok(response)
    }

    /// Send message to user
    /// pusher will procedure this to send message to user
    async fn send_msg_to_user(
        &self,
        request: Request<SendMsgRequest>,
    ) -> Result<Response<SendMsgResponse>, Status> {
        let msg = request
            .into_inner()
            .message
            .ok_or_else(|| Status::invalid_argument("message is empty"))?;
        debug!("send message to user: {:?}", msg);
        self.manager.send_single_msg(&msg.receiver_id, &msg).await;
        let response = Response::new(SendMsgResponse {});
        Ok(response)
    }

    async fn send_group_msg_to_user(
        &self,
        request: Request<SendGroupMsgRequest>,
    ) -> Result<Response<SendMsgResponse>, Status> {
        let req = request.into_inner();
        let msg = req
            .message
            .ok_or_else(|| Status::invalid_argument("message is empty"))?;
        let members_id = req.members_id;
        self.manager.send_group(&members_id, msg).await;
        let response = Response::new(SendMsgResponse {});
        Ok(response)
    }
}
