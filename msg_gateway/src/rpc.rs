use std::result::Result;

use synapse::health::{HealthServer, HealthService};
use tonic::transport::Server;
use tonic::{async_trait, Request, Response, Status};
use tracing::{debug, info};

use abi::config::{Component, Config};
use abi::errors::Error;
use abi::message::msg_service_server::MsgServiceServer;
use abi::message::{
    msg_service_server::MsgService, SendGroupMsgRequest, SendMsgRequest, SendMsgResponse,
};

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
        utils::register_service(config, Component::MessageGateway).await?;
        info!("<ws> rpc service register to service register center");

        // open health check
        let health_service = HealthServer::new(HealthService::new());
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
            .ok_or(Status::invalid_argument("message is empty"))?;
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
            .ok_or(Status::invalid_argument("message is empty"))?;
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
            .ok_or(Status::invalid_argument("message is empty"))?;
        let members = req.members;
        self.manager.send_group(members, msg).await;
        let response = Response::new(SendMsgResponse {});
        Ok(response)
    }
}
