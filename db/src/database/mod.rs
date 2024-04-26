mod friend;
mod group;
mod message;
mod mongodb;
mod postgres;
mod user;

use crate::database::friend::FriendRepo;
use abi::config::Config;
use cache::Cache;
use sqlx::PgPool;
use std::sync::Arc;
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
    pub friend: Box<dyn FriendRepo>,
}

impl DbRepo {
    pub async fn new(config: &Config) -> Self {
        let pool = PgPool::connect(&config.db.postgres.url()).await.unwrap();

        let msg = Box::new(postgres::PostgresMessage::new(pool.clone()));
        let user = Box::new(postgres::PostgresUser::new(pool.clone()));
        let friend = Box::new(postgres::PostgresFriend::new(pool.clone()));
        let group = Box::new(postgres::PostgresGroup::new(pool));

        Self {
            msg,
            group,
            user,
            friend,
        }
    }
}

pub async fn msg_rec_box_repo(config: &Config, cache: Arc<dyn Cache>) -> Arc<dyn MsgRecBoxRepo> {
    Arc::new(mongodb::MsgBox::from_config(config, cache).await)
}
