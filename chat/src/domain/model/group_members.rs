use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{Error, FromRow, Row};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct GroupMember {
    #[serde(skip)]
    pub id: i64,
    pub user_id: String,
    pub group_id: String,
    pub group_name: Option<String>,
    pub group_remark: Option<String>,
    pub delivered: bool,
    pub joined_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct GroupMemberWithUser {
    pub id: i64,
    pub user_id: String,
    pub group_id: String,
    pub avatar: String,
    pub gender: String,
    pub age: i32,
    pub region: Option<String>,
    pub group_name: Option<String>,
    pub joined_at: chrono::NaiveDateTime,
}

// implement slqx FromRow trait
impl FromRow<'_, PgRow> for GroupMemberWithUser {
    fn from_row(row: &PgRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.get("id"),
            user_id: row.get("user_id"),
            group_id: row.get("group_id"),
            avatar: row.get("avatar"),
            gender: row.get("gender"),
            age: row.get("age"),
            region: row.get("region"),
            group_name: row.get("group_name"),
            joined_at: row.get("joined_at"),
        })
    }
}
impl FromRow<'_, PgRow> for GroupMember {
    fn from_row(row: &PgRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.get("id"),
            user_id: row.get("user_id"),
            group_id: row.get("group_id"),
            group_name: row.get("group_name"),
            group_remark: row.get("group_remark"),
            delivered: row.get("delivered"),
            joined_at: row.get("joined_at"),
        })
    }
}
