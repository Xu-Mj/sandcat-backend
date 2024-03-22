use std::result::Result;

use abi::msg::{msg_service_server::MsgService, SendMsgRequest, SendMsgResponse};
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
        let msg = msg.unwrap().msg;
        if msg.is_none() {
            return Err(tonic::Status::invalid_argument("message is empty"));
        }
        self.manager.broadcast(msg.unwrap()).await?;
        let response = tonic::Response::new(SendMsgResponse {});
        Ok(response)
    }
}
