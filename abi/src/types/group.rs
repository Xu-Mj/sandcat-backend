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
            user_id: row.try_get("user_id")?,
            group_id: row.try_get("group_id")?,
            avatar: row.try_get("avatar")?,
            gender: row.try_get("gender")?,
            is_friend: false,
            age: row.try_get("age")?,
            region: row.try_get("region")?,
            group_name: row.try_get("group_name")?,
            joined_at: row.try_get("joined_at")?,
            remark: None,
            signature: row.try_get("signature")?,
        })
    }
}

// implement slqx FromRow trait
impl FromRow<'_, PgRow> for GroupInfo {
    fn from_row(row: &PgRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            owner: row.try_get("owner")?,
            avatar: row.try_get("avatar")?,
            description: row.try_get("description")?,
            announcement: row.try_get("announcement")?,
            create_time: row.try_get("create_time")?,
            update_time: row.try_get("update_time")?,
        })
    }
}
