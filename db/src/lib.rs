mod relation_db;

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::mpsc::Receiver;
use tonic::server::NamedService;
use tonic::transport::Server;
use tonic::{async_trait, Request, Response, Status};
use tracing::{debug, info};

use abi::config::Config;
use abi::errors::Error;
use abi::message::db_service_server::{DbService, DbServiceServer};
use abi::message::{GetDbMsgRequest, MsgToDb, SaveMessageRequest, SaveMessageResponse};
use utils::typos::{GrpcHealthCheck, Registration};

use crate::relation_db::{MsgRecBoxRepo, MsgStoreRepo};

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

    type GetMessagesStream = Pin<Box<dyn Stream<Item = Result<MsgToDb, Status>> + Send>>;

    async fn get_messages(
        &self,
        request: Request<GetDbMsgRequest>,
    ) -> Result<Response<Self::GetMessagesStream>, Status> {
        let req = request.into_inner();
        let result = self
            .mongodb
            .get_messages(req.start, req.end, "".to_string())
            .await?;
        Ok(Response::new(Box::pin(TonicReceiverStream::new(result))))
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
        // register service
        Self::register_service(config).await?;
        info!("<db> rpc service health check started");

        // open health check
        let (mut reporter, health_service) = tonic_health::server::health_reporter();
        reporter
            .set_serving::<DbServiceServer<DbRpcService>>()
            .await;
        info!("<db> rpc service register to service register center");

        let db_rpc = DbRpcService::new(config).await;
        let service = DbServiceServer::new(db_rpc);
        info!(
            "<db> rpc service started at {}",
            config.rpc.db.rpc_server_url()
        );

        Server::builder()
            .add_service(health_service)
            .add_service(service)
            .serve(config.rpc.db.rpc_server_url().parse().unwrap())
            .await
            .unwrap();
        Ok(())
    }

    async fn register_service(config: &Config) -> Result<(), Error> {
        // register service to service register center
        let center = utils::service_register_center(config);
        let grpc = format!(
            "{}/{}",
            config.rpc.db.rpc_server_url(),
            <DbServiceServer<DbRpcService> as NamedService>::NAME
        );
        let check = GrpcHealthCheck {
            name: config.rpc.db.name.clone(),
            grpc,
            grpc_use_tls: config.rpc.db.grpc_health_check.grpc_use_tls,
            interval: format!("{}s", config.rpc.db.grpc_health_check.interval),
        };
        let registration = Registration {
            id: format!("{}-{}", utils::get_host_name()?, &config.rpc.db.name),
            name: config.rpc.db.name.clone(),
            address: config.rpc.db.host.clone(),
            port: config.rpc.db.port,
            tags: config.rpc.db.tags.clone(),
            check: Some(check),
        };
        center.register(registration).await?;
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

/// implement the Stream for tokio::sync::mpsc::Receiver
pub struct TonicReceiverStream<T> {
    inner: Receiver<Result<T, Error>>,
}

impl<T> TonicReceiverStream<T> {
    pub fn new(inner: Receiver<Result<T, Error>>) -> Self {
        Self { inner }
    }
}

impl<T> Stream for TonicReceiverStream<T> {
    type Item = Result<T, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.poll_recv(cx) {
            Poll::Ready(Some(Ok(item))) => Poll::Ready(Some(Ok(item))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_rpc_service() {
        println!(
            "test_db_rpc_service:{}",
            <DbServiceServer<DbRpcService> as NamedService>::NAME
        )
    }
}
