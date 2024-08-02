// use std::sync::Arc;

// use synapse::health::{HealthServer, HealthService};

// use tonic::transport::Server;
// use tracing::info;

// use abi::config::{Component, Config};
// use abi::errors::Error;
// use abi::message::db_service_server::DbServiceServer;
// use abi::message::{GroupMemSeq, Msg, MsgType};
// use cache::Cache;

// use crate::{msg_rec_box_repo, DbRepo, MsgRecBoxRepo};

// pub mod service;
// pub use service::*;

// /// DbRpcService contains the postgres trait, mongodb trait and redis trait
// pub struct DbRpcService {
//     db: Arc<DbRepo>,
//     msg_rec_box: Arc<dyn MsgRecBoxRepo>,
//     cache: Arc<dyn Cache>,
// }

// impl DbRpcService {
//     pub async fn new(config: &Config) -> Self {
//         let cache = cache::cache(config);
//         Self {
//             db: Arc::new(DbRepo::new(config).await),
//             msg_rec_box: msg_rec_box_repo(config).await,
//             cache,
//         }
//     }

//     pub async fn start(config: &Config) {
//         // register service
//         utils::register_service(config, Component::MessageServer)
//             .await
//             .unwrap();
//         info!("<db> rpc service health check started");

//         // open health check
//         // let (mut reporter, health_service) = tonic_health::server::health_reporter();
//         // reporter
//         //     .set_serving::<DbServiceServer<DbRpcService>>()
//         //     .await;
//         let health_service = HealthServer::new(HealthService {});
//         info!("<db> rpc service register to service register center");

//         let db_rpc = DbRpcService::new(config).await;

//         let service = DbServiceServer::new(db_rpc);
//         info!(
//             "<db> rpc service started at {}",
//             config.rpc.db.rpc_server_url()
//         );

//         Server::builder()
//             .add_service(health_service)
//             .add_service(service)
//             .serve(config.rpc.db.rpc_server_url().parse().unwrap())
//             .await
//             .unwrap();
//     }

//     pub async fn handle_message(&self, message: Msg, need_to_history: bool) -> Result<(), Error> {
//         // task 1 save message to postgres

//         let mut tasks = Vec::with_capacity(2);
//         if !need_to_history {
//             let db = self.db.clone();
//             let cloned_msg = message.clone();
//             let db_task = tokio::spawn(async move {
//                 if let Err(e) = db.msg.save_message(cloned_msg).await {
//                     tracing::error!("save message to db failed: {}", e);
//                 }
//             });
//             tasks.push(db_task);
//         }

//         // task 2 save message to mongodb
//         let msg_rec_box = self.msg_rec_box.clone();
//         let msg_rec_box_task = tokio::spawn(async move {
//             // if the message type is friendship/group-operation delivery, we should delete it from mongodb
//             if message.msg_type == MsgType::GroupDismissOrExitReceived as i32
//                 || message.msg_type == MsgType::GroupInvitationReceived as i32
//                 || message.msg_type == MsgType::FriendshipReceived as i32
//             {
//                 if let Err(e) = msg_rec_box.delete_message(&message.server_id).await {
//                     tracing::error!("delete message from mongodb failed: {}", e);
//                 }
//                 return;
//             }
//             if let Err(e) = msg_rec_box.save_message(&message).await {
//                 tracing::error!("save message to mongodb failed: {}", e);
//             }
//         });
//         tasks.push(msg_rec_box_task);

//         // wait all tasks
//         futures::future::try_join_all(tasks)
//             .await
//             .map_err(Error::internal)?;
//         Ok(())
//     }

//     pub async fn handle_group_message(
//         &self,
//         message: Msg,
//         need_to_history: bool,
//         members: Vec<GroupMemSeq>,
//     ) -> Result<(), Error> {
//         // task 1 save message to postgres
//         let db = self.db.clone();
//         // update the user's seq in postgres
//         let need_update = members
//             .iter()
//             .enumerate()
//             .filter_map(|(index, item)| {
//                 if item.need_update {
//                     members.get(index).map(|v| v.mem_id.clone())
//                 } else {
//                     None
//                 }
//             })
//             .collect::<Vec<String>>();

//         let cloned_msg = if need_to_history {
//             Some(message.clone())
//         } else {
//             None
//         };

//         let db_task = tokio::spawn(async move {
//             if !need_update.is_empty() {
//                 if let Err(err) = db.seq.save_max_seq_batch(&need_update).await {
//                     tracing::error!("save max seq batch failed: {}", err);
//                     return Err(err);
//                 };
//             }

//             if let Some(cloned_msg) = cloned_msg {
//                 if let Err(e) = db.msg.save_message(cloned_msg).await {
//                     tracing::error!("save message to db failed: {}", e);
//                     return Err(e);
//                 }
//             }
//             Ok(())
//         });

//         // task 2 save message to mongodb
//         let msg_rec_box = self.msg_rec_box.clone();
//         let msg_rec_box_task = tokio::spawn(async move {
//             if let Err(e) = msg_rec_box.save_group_msg(message, members).await {
//                 tracing::error!("save message to mongodb failed: {}", e);
//                 return Err(e);
//             }
//             Ok(())
//         });

//         // wait all tasks complete
//         let (db_result, msg_rec_box_result) =
//             tokio::try_join!(db_task, msg_rec_box_task).map_err(Error::internal)?;

//         db_result?;
//         msg_rec_box_result?;

//         Ok(())
//     }
// }
