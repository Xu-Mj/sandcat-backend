use std::result::Result;

use abi::message::{msg_service_server::MsgService, SendMsgRequest, SendMsgResponse};
use tonic::async_trait;

use crate::manager::Manager;

pub struct MsgRpcService {
    manager: Manager,
}

impl MsgRpcService {
    pub fn new(manager: Manager) -> Self {
        Self { manager }
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
        self.manager.send_msg(msg).await;
        let response = tonic::Response::new(SendMsgResponse {});
        Ok(response)
    }
}
