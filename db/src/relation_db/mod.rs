mod message;
mod mongodb;
mod postgres;

use abi::config::Config;

pub use message::*;

pub async fn msg_store_repo(config: &Config) -> Box<dyn MsgStoreRepo> {
    Box::new(postgres::PostgresMessage::new(config).await)
}

pub async fn msg_rec_box_repo(config: &Config) -> Box<dyn MsgRecBoxRepo> {
    Box::new(mongodb::MsgBox::from_config(config).await)
}
