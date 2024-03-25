mod relation_db;

use crate::relation_db::{MsgRecBoxRepo, MsgStoreRepo};
use abi::config::Config;
use abi::errors::Error;
use abi::message::db_service_server::{DbService, DbServiceServer};
use abi::message::{MsgToDb, SaveMessageRequest, SaveMessageResponse};
use std::sync::Arc;
use tonic::transport::Server;
use tonic::{async_trait, Request, Response, Status};
use tracing::debug;

/// DbRpcService contains the postgres trait, mongodb trait and redis trait
pub struct DbRpcService {
    db: Arc<Box<dyn MsgStoreRepo>>,
    mongodb: Arc<Box<dyn MsgRecBoxRepo>>,
}

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
    pub async fn new(config: &Config) -> Self {
        Self {
            db: Arc::new(relation_db::msg_store_repo(config).await),
            mongodb: Arc::new(relation_db::msg_rec_box_repo(config).await),
        }
    }

    pub async fn start(config: &Config) -> Result<(), Error> {
        let service = DbServiceServer::new(Self::new(config).await);
        Server::builder()
            .add_service(service)
            .serve(config.rpc.db.rpc_server_url().parse().unwrap())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn handle_message(&self, message: MsgToDb) -> Result<(), Error> {
        // task 1 save message to postgres
        self.db.save_message(message.clone()).await?;
        // task 2 save message to mongodb
        // todo think about if the collection name should be here
        self.mongodb.save_message(message, "".to_string()).await?;

        Ok(())
    }
}
