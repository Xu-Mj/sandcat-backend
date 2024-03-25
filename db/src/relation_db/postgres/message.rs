use crate::relation_db::message::MsgStoreRepo;
use abi::config::Config;
use abi::errors::Error;
use abi::message::MsgToDb;
use async_trait::async_trait;
use sqlx::PgPool;

pub struct PostgresMessage {
    pool: PgPool,
}

impl PostgresMessage {
    pub async fn new(config: &Config) -> Self {
        let pool = PgPool::connect(&config.db.postgres.url()).await.unwrap();

        Self { pool }
    }
}
#[async_trait]
impl MsgStoreRepo for PostgresMessage {
    async fn save_message(&self, message: MsgToDb) -> Result<(), Error> {
        sqlx::query("INSERT INTO messages (local_id, server_id, content, send_id, receiver_id, content_type, send_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
            .bind(&message.local_id)
            .bind(&message.server_id)
            .bind(&message.content)
            .bind(&message.send_id)
            .bind(&message.receiver_id)
            .bind(message.content_type)
            .bind(message.send_time)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
