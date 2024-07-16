use async_trait::async_trait;
use tokio::sync::mpsc;

use abi::errors::Error;
use abi::message::{GroupMemSeq, Msg};

/// face to postgres db
#[async_trait]
pub trait MsgStoreRepo: Sync + Send {
    /// save message to db
    async fn save_message(&self, message: Msg) -> Result<(), Error>;
}

/// message receive box
/// face to mongodb
/// when user received message, will delete message from receive box
#[async_trait]
pub trait MsgRecBoxRepo: Sync + Send {
    /// save message, need message structure
    async fn save_message(&self, message: &Msg) -> Result<(), Error>;

    /// save message to message receive box
    /// need the group members id
    async fn save_group_msg(&self, message: Msg, members: Vec<GroupMemSeq>) -> Result<(), Error>;

    async fn delete_message(&self, message_id: &str) -> Result<(), Error>;

    async fn delete_messages(&self, user_id: &str, msg_seq: Vec<i64>) -> Result<(), Error>;

    #[allow(dead_code)]
    async fn get_message(&self, message_id: &str) -> Result<Option<Msg>, Error>;

    /// need to think about how to get message from receive box,
    /// use stream? or use pagination? prefer stream
    async fn get_messages_stream(
        &self,
        user_id: &str,
        start: i64,
        end: i64,
    ) -> Result<mpsc::Receiver<Result<Msg, Error>>, Error>;

    #[deprecated]
    async fn get_messages(&self, user_id: &str, start: i64, end: i64) -> Result<Vec<Msg>, Error>;

    async fn get_msgs(
        &self,
        user_id: &str,
        send_start: i64,
        send_end: i64,
        rec_start: i64,
        rec_end: i64,
    ) -> Result<Vec<Msg>, Error>;

    /// update message read status by user id and message sequence
    async fn msg_read(&self, user_id: &str, msg_seq: &[i64]) -> Result<(), Error>;
}
