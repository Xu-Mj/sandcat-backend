use std::result::Result;
use synapse::pb::service_registry_client::ServiceRegistryClient;
use synapse::pb::{HealthCheck, ServiceInstance};

use abi::config::Config;
use abi::errors::Error;
use abi::message::msg_service_server::MsgServiceServer;
use abi::message::{
    msg_service_server::MsgService, SendGroupMsgRequest, SendMsgRequest, SendMsgResponse,
};
use tonic::transport::{Channel, Server};
use tonic::{async_trait, Request, Response, Status};
use tracing::{debug, info};

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
        let health_service = synapse::pb::health_server::HealthServer::new(
            synapse::health_service::HealthService {},
        );
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
        let addr = format!(
            "{}://{}:{}",
            config.service_center.protocol, config.service_center.host, config.service_center.port
        );
        let channel = Channel::from_shared(addr).unwrap().connect().await.unwrap();
        let mut client = ServiceRegistryClient::new(channel);
        let service = ServiceInstance {
            id: format!("{}-{}", utils::get_host_name()?, &config.rpc.ws.name),
            name: config.rpc.ws.name.clone(),
            address: config.rpc.ws.host.clone(),
            port: config.rpc.ws.port as i32,
            tags: config.rpc.ws.tags.clone(),
            version: "".to_string(),
            r#type: 0,
            metadata: Default::default(),
            health_check: Some(HealthCheck {
                endpoint: "".to_string(),
                interval: 10,
                timeout: 10,
                retries: 10,
            }),
            status: 0,
        };
        client.register_service(service).await.unwrap();
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
