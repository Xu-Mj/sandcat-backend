use sqlx::postgres::PgRow;
use sqlx::{Error, FromRow, Row};
use tonic::Status;

use crate::message::{
    GetGroupAndMembersResp, GetMemberReq, GroupInfo, GroupMemSeq, GroupMember, GroupMemberRole,
    GroupMembersIdRequest, RemoveMemberRequest,
};

use super::Validator;

impl GroupMembersIdRequest {
    pub fn new(group_id: String) -> Self {
        Self { group_id }
    }
}

impl Validator for GetMemberReq {
    fn validate(&self) -> Result<(), Status> {
        if self.group_id.is_empty() {
            return Err(Status::invalid_argument("group_id is empty"));
        }
        if self.user_id.is_empty() {
            return Err(Status::invalid_argument("user_id is empty"));
        }
        if self.mem_ids.is_empty() {
            return Err(Status::invalid_argument("mem_ids is empty"));
        }
        Ok(())
    }
}

impl Validator for RemoveMemberRequest {
    fn validate(&self) -> Result<(), Status> {
        if self.group_id.is_empty() {
            return Err(Status::invalid_argument("group_id is empty"));
        }
        if self.user_id.is_empty() {
            return Err(Status::invalid_argument("user_id is empty"));
        }
        if self.mem_id.is_empty() {
            return Err(Status::invalid_argument("mem_ids is empty"));
        }
        Ok(())
    }
}

impl GetGroupAndMembersResp {
    pub fn new(group: GroupInfo, members: Vec<GroupMember>) -> Self {
        Self {
            group: Some(group),
            members,
        }
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
