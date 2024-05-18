use std::sync::Arc;

use synapse::health::{HealthServer, HealthService};

use tonic::transport::Server;
use tracing::info;

use abi::config::{Component, Config};
use abi::errors::Error;
use abi::message::db_service_server::DbServiceServer;
use abi::message::{Msg, MsgType};
use cache::Cache;

use crate::database;
use crate::database::{DbRepo, MsgRecBoxRepo};

pub mod service;
pub use service::*;

/// DbRpcService contains the postgres trait, mongodb trait and redis trait
pub struct DbRpcService {
    db: Arc<DbRepo>,
    msg_rec_box: Arc<dyn MsgRecBoxRepo>,
    cache: Arc<dyn Cache>,
}

impl DbRpcService {
    pub async fn new(config: &Config) -> Self {
        let cache = cache::cache(config);
        Self {
            db: Arc::new(DbRepo::new(config).await),
            msg_rec_box: database::msg_rec_box_repo(config, cache.clone()).await,
            cache,
        }
    }

    pub async fn start(config: &Config) {
        // register service
        utils::register_service(config, Component::Db)
            .await
            .unwrap();
        info!("<db> rpc service health check started");

        // open health check
        // let (mut reporter, health_service) = tonic_health::server::health_reporter();
        // reporter
        //     .set_serving::<DbServiceServer<DbRpcService>>()
        //     .await;
        let health_service = HealthServer::new(HealthService {});
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
    }

    pub async fn handle_message(&self, message: Msg, need_to_history: bool) -> Result<(), Error> {
        // task 1 save message to postgres
        let db = self.db.clone();
        let cloned_msg = message.clone();
        let db_task = tokio::spawn(async move {
            if !need_to_history {
                return;
            }
            if let Err(e) = db.msg.save_message(cloned_msg).await {
                tracing::error!("save message to db failed: {}", e);
            }
        });

        // task 2 save message to mongodb
        let msg_rec_box = self.msg_rec_box.clone();
        let msg_rec_box_task = tokio::spawn(async move {
            // if the message type is friendship/group-operation delivery, we should delete it from mongodb
            if message.msg_type == MsgType::GroupDismissOrExitReceived as i32
                || message.msg_type == MsgType::GroupInvitationReceived as i32
                || message.msg_type == MsgType::FriendshipReceived as i32
            {
                if let Err(e) = msg_rec_box.delete_message(&message.server_id).await {
                    tracing::error!("delete message from mongodb failed: {}", e);
                }
                return;
            }
            if let Err(e) = msg_rec_box.save_message(&message).await {
                tracing::error!("save message to mongodb failed: {}", e);
            }
        });
        tokio::try_join!(db_task, msg_rec_box_task)
            .map_err(|e| Error::InternalServer(e.to_string()))?;
        Ok(())
    }

    pub async fn handle_group_message(
        &self,
        message: Msg,
        need_to_history: bool,
        members_id: Vec<String>,
    ) -> Result<(), Error> {
        // task 1 save message to postgres
        let db = self.db.clone();
        let cloned_msg = message.clone();
        let db_task = tokio::spawn(async move {
            if !need_to_history {
                return;
            }
            if let Err(e) = db.msg.save_message(cloned_msg).await {
                tracing::error!("save message to db failed: {}", e);
            }
        });

        // task 2 save message to mongodb
        let msg_rec_box = self.msg_rec_box.clone();
        let msg_rec_box_task = tokio::spawn(async move {
            if let Err(e) = msg_rec_box.save_group_msg(message, members_id).await {
                tracing::error!("save message to mongodb failed: {}", e);
            }
        });
        tokio::try_join!(db_task, msg_rec_box_task)
            .map_err(|e| Error::InternalServer(e.to_string()))?;
        Ok(())
    }
}
