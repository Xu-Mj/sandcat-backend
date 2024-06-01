use abi::errors::Error;
use tonic::async_trait;

#[async_trait]
pub trait SeqRepo: Sync + Send {
    async fn save_max_seq(&self, user_id: i64) -> Result<i64, Error>;
}
