mod group;
mod message;
mod mongodb;
mod postgres;

use abi::config::Config;
// use sqlx::PgPool;

pub(crate) use crate::relation_db::group::GroupStoreRepo;
pub(crate) use crate::relation_db::message::{MsgRecBoxRepo, MsgStoreRepo};

/// shall we create a structure to hold everything we need?
/// like db pool and mongodb's database
// pub struct DbRepo {
//     pub msg: Box<dyn MsgStoreRepo>,
//     pub msg_rec_box: Box<dyn MsgRecBoxRepo>,
//     pub group: Box<dyn GroupStoreRepo>,
// }
//
// rpc DbRepo {
//     pub async fn new(config: &Config) -> Self {
//         let pool = PgPool::connect(&config.db.postgres.url()).await.unwrap();
//
//         let msg = Box::new(postgres::PostgresMessage::new(pool.clone()).await);
//         let msg_rec_box = msg_rec_box_repo(config).await;
//         let group = Box::new(postgres::PostgresGroup::new(pool).await);
//         Self {
//             msg,
//             msg_rec_box,
//             group,
//         }
//     }
// }

pub async fn msg_store_repo(config: &Config) -> Box<dyn MsgStoreRepo> {
    Box::new(postgres::PostgresMessage::from_config(config).await)
}

pub async fn msg_rec_box_repo(config: &Config) -> Box<dyn MsgRecBoxRepo> {
    Box::new(mongodb::MsgBox::from_config(config).await)
}

pub async fn group_repo(config: &Config) -> Box<dyn GroupStoreRepo> {
    Box::new(postgres::PostgresGroup::from_config(config).await)
}
