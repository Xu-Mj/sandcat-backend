use async_trait::async_trait;
use sqlx::PgPool;

use abi::errors::Error;
use abi::message::Msg;

use crate::message::MsgStoreRepo;

#[derive(Debug)]
pub struct PostgresMessage {
    pool: PgPool,
}

impl PostgresMessage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MsgStoreRepo for PostgresMessage {
    async fn save_message(&self, message: Msg) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO messages
             (client_id, server_id, sender_id, receiver_id, msg_type, content_type, content, send_time, platform, group_id)
             VALUES
             ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT DO NOTHING",
        )
        .bind(&message.client_id)
        .bind(&message.server_id)
        .bind(&message.sender_id)
        .bind(&message.receiver_id)
        .bind(message.msg_type)
        .bind(message.content_type)
        .bind(&message.content)
        .bind(message.send_time)
        .bind(message.platform)
        .bind(&message.group_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
