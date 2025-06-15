use sqlx::postgres::PgRow;
use sqlx::{Error, FromRow, Row};
use tonic::Status;

use crate::message::{
    GetGroupAndMembersResp, GetMemberReq, GroupAnnouncement, GroupCategory, GroupFile, GroupInfo,
    GroupMemSeq, GroupMember, GroupMemberRole, GroupMembersIdRequest, GroupMuteRecord, GroupPoll,
    RemoveMemberRequest,
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
            is_muted: row.try_get("is_muted")?,
            notification_level: row.try_get("notification_level")?,
            last_read_time: row.try_get("last_read_time")?,
            display_order: row.try_get("display_order")?,
            joined_by: row.try_get("joined_by")?,
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
            max_members: row.try_get("max_members")?,
            is_public: row.try_get("is_public")?,
            join_approval_required: row.try_get("join_approval_required")?,
            category: row.try_get("category")?,
            tags: row.try_get("tags")?,
            mute_all: row.try_get("mute_all")?,
            only_admin_post: row.try_get("only_admin_post")?,
            invite_permission: row.try_get("invite_permission")?,
            pinned_messages: row.try_get("pinned_messages")?,
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

// GroupCategory 的 FromRow 实现
impl FromRow<'_, PgRow> for GroupCategory {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id").unwrap_or_default(),
            name: row.try_get("name").unwrap_or_default(),
            description: row.try_get("description").unwrap_or_default(),
            display_order: row.try_get("display_order").unwrap_or_default(),
            create_time: row.try_get("create_time").unwrap_or_default(),
            update_time: row.try_get("update_time").unwrap_or_default(),
        })
    }
}

// GroupFile 的 FromRow 实现
impl FromRow<'_, PgRow> for GroupFile {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id").unwrap_or_default(),
            group_id: row.try_get("group_id").unwrap_or_default(),
            uploader_id: row.try_get("uploader_id").unwrap_or_default(),
            file_name: row.try_get("file_name").unwrap_or_default(),
            file_url: row.try_get("file_url").unwrap_or_default(),
            file_size: row.try_get("file_size").unwrap_or_default(),
            file_type: row.try_get("file_type").unwrap_or_default(),
            upload_time: row.try_get("upload_time").unwrap_or_default(),
            is_pinned: row.try_get("is_pinned").unwrap_or_default(),
            download_count: row.try_get("download_count").unwrap_or_default(),
            thumbnail_url: row.try_get("thumbnail_url").unwrap_or_default(),
        })
    }
}

// GroupPoll 的 FromRow 实现
impl FromRow<'_, PgRow> for GroupPoll {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        let options_json: String = row.try_get("options").unwrap_or_default();

        Ok(Self {
            id: row.try_get("id").unwrap_or_default(),
            group_id: row.try_get("group_id").unwrap_or_default(),
            creator_id: row.try_get("creator_id").unwrap_or_default(),
            title: row.try_get("title").unwrap_or_default(),
            options: serde_json::from_str(&options_json).unwrap_or_default(),
            is_multiple: row.try_get("is_multiple").unwrap_or_default(),
            is_anonymous: row.try_get("is_anonymous").unwrap_or_default(),
            created_at: row.try_get("created_at").unwrap_or_default(),
            expires_at: row.try_get("expires_at").unwrap_or_default(),
            status: row.try_get("status").unwrap_or_default(),
        })
    }
}

// GroupMuteRecord 的 FromRow 实现
impl FromRow<'_, PgRow> for GroupMuteRecord {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id").unwrap_or_default(),
            group_id: row.try_get("group_id").unwrap_or_default(),
            user_id: row.try_get("user_id").unwrap_or_default(),
            operator_id: row.try_get("operator_id").unwrap_or_default(),
            mute_until: row.try_get("mute_until").unwrap_or_default(),
            reason: row.try_get("reason").ok().unwrap_or_default(),
            created_at: row.try_get("created_at").unwrap_or_default(),
        })
    }
}

// GroupAnnouncement 的 FromRow 实现
impl FromRow<'_, PgRow> for GroupAnnouncement {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id").unwrap_or_default(),
            group_id: row.try_get("group_id").unwrap_or_default(),
            creator_id: row.try_get("creator_id").unwrap_or_default(),
            content: row.try_get("content").unwrap_or_default(),
            created_at: row.try_get("created_at").unwrap_or_default(),
            is_pinned: row.try_get("is_pinned").unwrap_or_default(),
        })
    }
}
