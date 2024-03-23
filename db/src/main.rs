use abi::config::Config;
use abi::message::db_service_server::{DbService, DbServiceServer};
use abi::message::{SaveMessageRequest, SaveMessageResponse};
use tonic::{async_trait, Request, Response, Status};
use tracing::{debug, info, Level};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    let config = Config::load("./abi/fixtures/im.yml").unwrap();
    let rpc_addr = format!("{}:{}", config.rpc.db.host, config.rpc.db.port);
    info!("db rpc addr: {}", rpc_addr);
    tonic::transport::Server::builder()
        .add_service(DbServiceServer::new(DbRpcService {}))
        .serve(rpc_addr.parse().unwrap())
        .await
        .unwrap();
}

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
