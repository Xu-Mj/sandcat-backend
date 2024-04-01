mod group;
mod message;
mod mongodb;
mod postgres;
mod user;

use abi::config::Config;
use sqlx::PgPool;
// use sqlx::PgPool;

pub(crate) use crate::database::group::GroupStoreRepo;
pub(crate) use crate::database::message::{MsgRecBoxRepo, MsgStoreRepo};
pub(crate) use crate::database::user::UserRepo;

/// shall we create a structure to hold everything we need?
/// like db pool and mongodb's database
pub struct DbRepo {
    pub msg: Box<dyn MsgStoreRepo>,
    pub group: Box<dyn GroupStoreRepo>,
    pub user: Box<dyn UserRepo>,
}

impl DbRepo {
    pub async fn new(config: &Config) -> Self {
        let pool = PgPool::connect(&config.db.postgres.url()).await.unwrap();

        let msg = Box::new(postgres::PostgresMessage::new(pool.clone()).await);
        let user = Box::new(postgres::PostgresUser::new(pool.clone()).await);
        let group = Box::new(postgres::PostgresGroup::new(pool).await);

        Self { msg, group, user }
    }
}

pub async fn msg_rec_box_repo(config: &Config) -> Box<dyn MsgRecBoxRepo> {
    Box::new(mongodb::MsgBox::from_config(config).await)
}
//
// pub async fn msg_store_repo(config: &Config) -> Box<dyn MsgStoreRepo> {
//     Box::new(postgres::PostgresMessage::from_config(config).await)
// }
//
// pub async fn group_repo(config: &Config) -> Box<dyn GroupStoreRepo> {
//     Box::new(postgres::PostgresGroup::from_config(config).await)
// }
//
// pub async fn user_repo(config: &Config) -> Box<dyn UserRepo> {
//     let pool = PgPool::connect(&config.db.postgres.url()).await.unwrap();
//
//     Box::new(postgres::PostgresUser::new(pool).await)
// }
