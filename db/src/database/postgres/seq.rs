use abi::errors::Error;
use sqlx::{PgPool, Row};
use tonic::async_trait;

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
    async fn save_max_seq(&self, user_id: &str) -> Result<i64, Error> {
        let max_seq = sqlx::query(
            "UPDATE sequence SET max_seq = max_seq + $1 WHERE user_id = $2 RETURNING max_seq",
        )
        .bind(self.seq_step)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?
        .try_get(0)?;
        Ok(max_seq)
    }
}
