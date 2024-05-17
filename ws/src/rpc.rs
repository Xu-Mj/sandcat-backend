use std::result::Result;
use std::time::Duration;

use synapse::health::{HealthCheck, HealthServer, HealthService};
use synapse::service::{Scheme, ServiceInstance, ServiceRegistryClient};
use tonic::transport::{Endpoint, Server};
use tonic::{async_trait, Request, Response, Status};
use tracing::{debug, info};

use abi::config::Config;
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
        Self::register_service(config).await?;
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
    async fn register_service(config: &Config) -> Result<(), Error> {
        // register service to service register center
        let addr = format!(
            "{}://{}:{}",
            config.service_center.protocol, config.service_center.host, config.service_center.port
        );
        let endpoint = Endpoint::from_shared(addr)
            .map_err(|e| Error::TonicError(e.to_string()))?
            .connect_timeout(Duration::from_secs(config.service_center.timeout));
        let mut client = ServiceRegistryClient::connect(endpoint)
            .await
            .map_err(|e| Error::TonicError(e.to_string()))?;
        let mut health_check = None;
        if config.rpc.health_check {
            health_check = Some(HealthCheck {
                endpoint: "".to_string(),
                interval: 10,
                timeout: 10,
                retries: 10,
                scheme: Scheme::from(config.rpc.ws.protocol.as_str()) as i32,
                tls_domain: None,
            });
        }
        let service = ServiceInstance {
            id: format!("{}-{}", utils::get_host_name()?, &config.rpc.ws.name),
            name: config.rpc.ws.name.clone(),
            address: config.rpc.ws.host.clone(),
            port: config.rpc.ws.port as i32,
            tags: config.rpc.ws.tags.clone(),
            version: "".to_string(),
            metadata: Default::default(),
            health_check,
            status: 0,
            scheme: Scheme::from(config.rpc.ws.protocol.as_str()) as i32,
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
