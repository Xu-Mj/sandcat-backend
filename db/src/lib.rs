use abi::config::Config;
use abi::errors::Error;
use abi::message::db_service_server::{DbService, DbServiceServer};
use abi::message::{SaveMessageRequest, SaveMessageResponse};
use tonic::transport::Server;
use tonic::{async_trait, Request, Response, Status};
use tracing::debug;

pub struct DbRpcService {}

#[async_trait]
impl DbService for DbRpcService {
    async fn save_message(
        &self,
        request: Request<SaveMessageRequest>,
    ) -> Result<Response<SaveMessageResponse>, Status> {
        let message = request.into_inner().message;
        if message.is_none() {
            return Err(Status::invalid_argument("message is empty"));
        }
        debug!("save message: {:?}", message.unwrap());
        return Ok(Response::new(SaveMessageResponse {}));
    }
}

impl DbRpcService {
    pub async fn start(config: &Config) -> Result<(), Error> {
        let service = DbServiceServer::new(Self {});
        Server::builder()
            .add_service(service)
            .serve(config.rpc.db.rpc_server_url().parse().unwrap())
            .await
            .unwrap();
        Ok(())
    }
}
