use abi::errors::Error;
use abi::message::{GroupInvitation, MsgToDb};
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
    async fn save_message(&self, message: MsgToDb) -> Result<(), Error>;

    /// save group invitation to message receive box
    async fn save_group_invitation(&self, group_invitation: GroupInvitation) -> Result<(), Error>;

    async fn delete_message(&self, message_id: &str) -> Result<(), Error>;

    async fn delete_messages(&self, message_ids: Vec<String>) -> Result<(), Error>;

    /// need to think about how to get message from receive box,
    /// use stream? or use pagination? prefer stream
    async fn get_message(&self, message_id: &str) -> Result<Option<MsgToDb>, Error>;

    async fn get_messages(
        &self,
        user_id: &str,
        start: i64,
        end: i64,
    ) -> Result<mpsc::Receiver<Result<MsgToDb, Error>>, Error>;
}
