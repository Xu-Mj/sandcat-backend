use abi::errors::Error;
use futures::TryStreamExt;
use sqlx::{PgPool, Row};
use tokio::sync::mpsc::{self, Receiver};
use tonic::async_trait;
use tracing::error;

use crate::database::seq::SeqRepo;

pub struct PostgresSeq {
    pool: PgPool,
    seq_step: i32,
}

impl PostgresSeq {
    pub fn new(pool: PgPool, seq_step: i32) -> Self {
        PostgresSeq { pool, seq_step }
    }
}

#[async_trait]
impl SeqRepo for PostgresSeq {
    async fn save_send_max_seq(&self, user_id: &str) -> Result<i64, Error> {
        let max_seq = sqlx::query(
            "UPDATE sequence SET send_max_seq = send_max_seq + $1 WHERE user_id = $2 RETURNING send_max_seq",
        )
        .bind(self.seq_step)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?
        .try_get(0)?;
        Ok(max_seq)
    }

    async fn save_max_seq(&self, user_id: &str) -> Result<i64, Error> {
        let max_seq = sqlx::query(
            "UPDATE sequence SET rec_max_seq = rec_max_seq + $1 WHERE user_id = $2 RETURNING rec_max_seq",
        )
        .bind(self.seq_step)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?
        .try_get(0)?;
        Ok(max_seq)
    }

    /// update max_seq in batch
    async fn save_max_seq_batch(&self, user_ids: &[String]) -> Result<(), Error> {
        if user_ids.is_empty() {
            return Ok(());
        }
        sqlx::query("UPDATE sequence SET rec_max_seq = rec_max_seq + $1 WHERE user_id = ANY($2)")
            .bind(self.seq_step)
            .bind(user_ids as &[_])
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_max_seq(&self) -> Result<Receiver<(String, i64, i64)>, Error> {
        let mut result = sqlx::query("SELECT user_id, send_max_seq, rec_max_seq FROM sequence")
            .fetch(&self.pool);
        let (tx, rx) = mpsc::channel(1024);
        while let Some(item) = result.try_next().await? {
            let user_id: String = item.try_get(0)?;
            let send_max_seq: i64 = item.try_get(1)?;
            let rec_max_seq: i64 = item.try_get(2)?;
            if let Err(e) = tx.send((user_id, send_max_seq, rec_max_seq)).await {
                error!("send error: {}", e);
            };
        }
        Ok(rx)
    }
}
