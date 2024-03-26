use abi::errors::Error;
use abi::message::MsgToDb;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// face to postgres db
#[async_trait]
pub trait MsgStoreRepo: Sync + Send {
    /// save message to db
    async fn save_message(&self, message: MsgToDb) -> Result<(), Error>;
}

/// message receive box
/// face to mongodb
/// when user received message, will delete message from receive box
#[async_trait]
pub trait MsgRecBoxRepo: Sync + Send {
    /// save message, need message structure and collection name
    async fn save_message(&self, message: MsgToDb, collection: String) -> Result<(), Error>;

    async fn delete_message(&self, message_id: String, collection: String) -> Result<(), Error>;

    async fn delete_messages(
        &self,
        message_ids: Vec<String>,
        collection: String,
    ) -> Result<(), Error>;

    /// need to think about how to get message from receive box,
    /// use stream? or use pagination? prefer stream
    async fn get_message(
        &self,
        message_id: String,
        collection: String,
    ) -> Result<Option<MsgToDb>, Error>;

    async fn get_messages(
        &self,
        start: i64,
        end: i64,
        collection: String,
    ) -> Result<mpsc::Receiver<Result<MsgToDb, Error>>, Error>;
}
