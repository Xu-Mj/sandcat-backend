use tonic::transport::{Channel, Server};
use tonic::{async_trait, Request, Response, Status};

use abi::config::Config;
use abi::errors::Error;
use abi::message::msg_service_client::MsgServiceClient;
use abi::message::push_service_server::{PushService, PushServiceServer};
use abi::message::{SendMsgRequest, SendMsgResponse};

pub struct PusherRpcService {
    ws_rpc: MsgServiceClient<Channel>,
}

impl PusherRpcService {
    pub async fn new(config: &Config) -> Self {
        let ws_rpc = Self::get_ws_rpc_client(config).await.unwrap();
        Self { ws_rpc }
    }

    pub async fn start(config: &Config) -> Result<(), Error> {
        let pusher_rpc = Self::new(config).await;
        let service = PushServiceServer::new(pusher_rpc);
        Server::builder()
            .add_service(service)
            .serve(config.rpc.pusher.rpc_server_url().parse().unwrap())
            .await
            .unwrap();
        Ok(())
    }

    async fn get_ws_rpc_client(config: &Config) -> Result<MsgServiceClient<Channel>, Error> {
        let ws_rpc = MsgServiceClient::connect(config.rpc.ws.url(false))
            .await
            .map_err(|e| Error::TonicError(e.to_string()))?;
        Ok(ws_rpc)
    }
}

#[async_trait]
impl PushService for PusherRpcService {
    async fn push_msg(
        &self,
        request: Request<SendMsgRequest>,
    ) -> Result<Response<SendMsgResponse>, Status> {
        // push message to ws
        let mut ws_rpc = self.ws_rpc.clone();
        ws_rpc.send_msg_to_user(request).await
    }
}
