use sqlx::postgres::PgRow;
use sqlx::{Error, FromRow, Row};

use crate::message::{
    GroupCreate, GroupCreateRequest, GroupDeleteRequest, GroupInfo, GroupMemSeq, GroupMember,
    GroupMemberRole, GroupMembersIdRequest, GroupUpdate, GroupUpdateRequest,
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
#[derive(sqlx::Type, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[sqlx(type_name = "group_role")]
pub enum GroupRole {
    Owner,
    Admin,
    Member,
}

impl From<GroupRole> for GroupMemberRole {
    fn from(value: GroupRole) -> Self {
        match value {
            GroupRole::Owner => Self::Owner,
            GroupRole::Admin => Self::Admin,
            GroupRole::Member => Self::Member,
        }
    }
}

// implement slqx FromRow trait
impl FromRow<'_, PgRow> for GroupMember {
    fn from_row(row: &PgRow) -> Result<Self, Error> {
        let role: GroupRole = row.try_get("role")?;
        let role = GroupMemberRole::from(role) as i32;
        Ok(Self {
            user_id: row.try_get("user_id")?,
            group_id: row.try_get("group_id")?,
            avatar: row.try_get("avatar")?,
            gender: row.try_get("gender")?,
            age: row.try_get("age")?,
            region: row.try_get("region")?,
            group_name: row.try_get("group_name")?,
            joined_at: row.try_get("joined_at")?,
            remark: None,
            signature: row.try_get("signature")?,
            role,
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

impl GroupMemSeq {
    pub fn new(mem_id: String, cur_seq: i64, max_seq: i64, need_update: bool) -> Self {
        Self {
            mem_id,
            cur_seq,
            max_seq,
            need_update,
        }
    }
}
