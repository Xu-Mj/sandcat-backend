use deadpool_diesel::postgres::Pool;
use diesel::{ExpressionMethods, Insertable, Queryable, RunQueryDsl, Selectable, SelectableHelper};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{Error, FromRow, PgPool, Row};

use crate::domain::model::msg::{
    ContentType, Hangup, InviteCancelMsg, InviteNotAnswerMsg, InviteType, Single,
};
use crate::infra::db::schema::messages;
use crate::infra::errors::{adapt_infra_error, InfraError};
use crate::utils;

#[derive(Clone, Default, Debug, Serialize, Deserialize, Queryable, Selectable)]
#[diesel(table_name=messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MsgDb {
    // pub msg_type: String,
    pub msg_id: String,
    pub content: String,
    pub send_id: String,
    pub friend_id: String,
    pub content_type: String,
    pub is_read: bool,
    pub delivered: bool,
    pub create_time: chrono::NaiveDateTime,
}

impl FromRow<'_, PgRow> for MsgDb {
    fn from_row(row: &'_ PgRow) -> Result<Self, Error> {
        Ok(Self {
            msg_id: row.get("msg_id"),
            content: row.get("content"),
            send_id: row.get("send_id"),
            friend_id: row.get("friend_id"),
            content_type: row.get("content_type"),
            is_read: row.get("is_read"),
            delivered: row.get("delivered"),
            create_time: row.get("create_time"),
        })
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name=messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewMsgDb {
    pub msg_id: String,
    pub content: String,
    pub send_id: String,
    pub friend_id: String,
    pub content_type: String,
    pub is_read: bool,
    pub delivered: bool,
    pub create_time: chrono::NaiveDateTime,
}

impl From<Single> for NewMsgDb {
    fn from(msg: Single) -> Self {
        Self {
            msg_id: msg.msg_id,
            content: msg.content,
            send_id: msg.send_id,
            friend_id: msg.friend_id,
            content_type: msg.content_type.to_string(),
            is_read: false,
            delivered: false,
            create_time: chrono::Local::now().naive_local(),
        }
    }
}

impl From<Hangup> for NewMsgDb {
    fn from(msg: Hangup) -> Self {
        let content_type = match msg.invite_type {
            InviteType::Video => ContentType::Video.to_string(),
            InviteType::Audio => ContentType::Audio.to_string(),
        };
        let content = utils::format_milliseconds(msg.sustain);
        Self {
            msg_id: msg.msg_id,
            content,
            send_id: msg.send_id,
            friend_id: msg.friend_id,
            content_type,
            is_read: false,
            delivered: false,
            create_time: chrono::Local::now().naive_local(),
        }
    }
}

impl From<InviteNotAnswerMsg> for NewMsgDb {
    fn from(msg: InviteNotAnswerMsg) -> Self {
        let content_type = match msg.invite_type {
            InviteType::Video => ContentType::Video.to_string(),
            InviteType::Audio => ContentType::Audio.to_string(),
        };
        Self {
            msg_id: msg.msg_id,
            content: "Not Answer".to_string(),
            send_id: msg.send_id,
            friend_id: msg.friend_id,
            content_type,
            is_read: false,
            delivered: false,
            create_time: chrono::Local::now().naive_local(),
        }
    }
}

impl From<InviteCancelMsg> for NewMsgDb {
    fn from(msg: InviteCancelMsg) -> Self {
        let content_type = match msg.invite_type {
            InviteType::Video => ContentType::Video.to_string(),
            InviteType::Audio => ContentType::Audio.to_string(),
        };
        Self {
            msg_id: msg.msg_id,
            content: "Canceled By Caller".to_string(),
            send_id: msg.send_id,
            friend_id: msg.friend_id,
            content_type,
            is_read: false,
            delivered: false,
            create_time: chrono::Local::now().naive_local(),
        }
    }
}

// 插入一条消息
pub async fn insert_msg(pool: &Pool, new_msg_db: NewMsgDb) -> Result<NewMsgDb, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    let msg_db = conn
        .interact(|conn| {
            diesel::insert_into(messages::table)
                .values(new_msg_db)
                .returning(NewMsgDb::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(msg_db)
}

// 获取离线消息
pub async fn get_offline_msg(pool: &PgPool, user_id: String) -> Result<Vec<MsgDb>, InfraError> {
    let msg_list =
        sqlx::query_as("SELECT * FROM messages WHERE friend_id = $1  AND delivered = false")
            .bind(user_id)
            .fetch_all(pool)
            .await?;
    /*    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    let msg_list = conn
        .interact(|conn| {
            messages::table
                .filter(
                    messages::friend_id.eq(user_id).and(
                        messages::is_read
                            .eq(false)
                            .and(messages::delivered.eq(false)),
                    ),
                )
                .select(MsgDb::as_select())
                .get_results(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;*/
    Ok(msg_list)
}

// 消息送达
pub async fn msg_delivered(pool: &PgPool, id: &str) -> Result<(), InfraError> {
    sqlx::query("UPDATE messages SET delivered = true WHERE msg_id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// 消息已读
pub async fn msg_read(pool: &Pool, ids: Vec<String>) -> Result<usize, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    let count = conn
        .interact(|conn| {
            diesel::update(messages::table)
                .filter(messages::msg_id.eq_any(ids))
                .set(messages::is_read.eq(true))
                .execute(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(count)
}
