use abi::errors::Error;
use tonic::async_trait;

#[async_trait]
pub trait SeqRepo: Sync + Send {
    async fn save_max_seq(&self, user_id: &str) -> Result<i64, Error>;
    async fn save_max_seq_batch(&self, user_ids: &[String]) -> Result<(), Error>;
}
