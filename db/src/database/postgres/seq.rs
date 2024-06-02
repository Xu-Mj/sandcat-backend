use abi::errors::Error;
use sqlx::{PgPool, Row};
use tonic::async_trait;

use crate::database::seq::SeqRepo;

pub struct PostgresSeq {
    pool: PgPool,
}

impl PostgresSeq {
    pub fn new(pool: PgPool) -> Self {
        PostgresSeq { pool }
    }
}

#[async_trait]
impl SeqRepo for PostgresSeq {
    async fn save_max_seq(&self, user_id: &str, seq_step: i32) -> Result<i64, Error> {
        let max_seq =
            sqlx::query("UPDATE sequence SET seq = seq + $1 WHERE user_id = $2 RETURNING seq")
                .bind(seq_step)
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?
                .try_get(0)?;
        Ok(max_seq)
    }
}
