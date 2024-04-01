use sqlx::postgres::PgRow;
use sqlx::{Error, FromRow, Row};

use crate::message::{
    GroupCreate, GroupCreateRequest, GroupDeleteRequest, GroupInfo, GroupMember,
    GroupMembersIdRequest, GroupUpdate, GroupUpdateRequest,
};

impl GroupCreateRequest {
    pub fn new(group: GroupCreate) -> Self {
        Self { group: Some(group) }
    }
}

impl GroupUpdateRequest {
    pub fn new(group: GroupUpdate) -> Self {
        Self { group: Some(group) }
    }
}

impl GroupDeleteRequest {
    pub fn new(group_id: String, user_id: String) -> Self {
        Self { group_id, user_id }
    }
}

impl GroupMembersIdRequest {
    pub fn new(group_id: String) -> Self {
        Self { group_id }
    }
}

// implement slqx FromRow trait
impl FromRow<'_, PgRow> for GroupMember {
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

// implement slqx FromRow trait
impl FromRow<'_, PgRow> for GroupInfo {
    fn from_row(row: &PgRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.get("id"),
            name: row.get("name"),
            owner: row.get("owner"),
            avatar: row.get("avatar"),
            description: row.get("description"),
            announcement: row.get("announcement"),
            create_time: row.get("create_time"),
            update_time: row.get("update_time"),
        })
    }
}
