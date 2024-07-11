use crate::message::{Friend, FriendDb, Friendship, FriendshipStatus, FriendshipWithUser, User};
use sqlx::postgres::PgRow;
use sqlx::{Error, FromRow, Row};
use std::fmt::{Display, Formatter};

impl Display for FriendshipStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FriendshipStatus::Pending => write!(f, "Pending"),
            FriendshipStatus::Accepted => write!(f, "Accepted"),
            FriendshipStatus::Rejected => write!(f, "Rejected"),
            FriendshipStatus::Blacked => write!(f, "Blacked"),
            FriendshipStatus::Deleted => write!(f, "Deleted"),
        }
    }
}
#[derive(sqlx::Type, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[sqlx(type_name = "friend_request_status")]
pub enum FsStatus {
    #[default]
    Pending,
    Accepted,
    Rejected,
    /// / blacklist
    Blacked,
    Deleted,
}

impl From<FsStatus> for FriendshipStatus {
    fn from(value: FsStatus) -> Self {
        match value {
            FsStatus::Pending => Self::Pending,
            FsStatus::Accepted => Self::Accepted,
            FsStatus::Rejected => Self::Rejected,
            FsStatus::Blacked => Self::Blacked,
            FsStatus::Deleted => Self::Deleted,
        }
    }
}

impl FromRow<'_, PgRow> for Friendship {
    fn from_row(row: &'_ PgRow) -> Result<Self, Error> {
        let status: FsStatus = row.try_get("status")?;
        let status = FriendshipStatus::from(status);
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            friend_id: row.try_get("friend_id")?,
            status: status as i32,
            apply_msg: row.try_get("apply_msg")?,
            req_remark: row.try_get("req_remark")?,
            resp_msg: row.try_get("resp_msg")?,
            resp_remark: row.try_get("resp_remark")?,
            source: row.try_get("source")?,
            create_time: row.try_get("create_time")?,
            update_time: row.try_get("update_time")?,
        })
    }
}

impl FromRow<'_, PgRow> for FriendDb {
    fn from_row(row: &'_ PgRow) -> Result<Self, Error> {
        let status: FsStatus = row.try_get("status")?;
        let status = FriendshipStatus::from(status);
        Ok(Self {
            id: row.try_get("id")?,
            fs_id: row.try_get("fs_id")?,
            user_id: row.try_get("user_id")?,
            friend_id: row.try_get("friend_id")?,
            status: status as i32,
            remark: row.try_get("resp_remark")?,
            source: row.try_get("source")?,
            create_time: row.try_get("create_time")?,
            update_time: row.try_get("update_time")?,
        })
    }
}

impl FromRow<'_, PgRow> for Friend {
    fn from_row(row: &'_ PgRow) -> Result<Self, Error> {
        let status: FsStatus = row.try_get("status").unwrap_or_default();
        let status = FriendshipStatus::from(status);
        Ok(Self {
            fs_id: row.try_get("fs_id").unwrap_or_default(),
            friend_id: row.try_get("friend_id").unwrap_or_default(),
            name: row.try_get("name").unwrap_or_default(),
            account: row.try_get("account").unwrap_or_default(),
            avatar: row.try_get("avatar").unwrap_or_default(),
            gender: row.try_get("gender").unwrap_or_default(),
            age: row.try_get("age").unwrap_or_default(),
            region: row.try_get("region").unwrap_or_default(),
            status: status as i32,
            remark: row.try_get("remark").unwrap_or_default(),
            source: row.try_get("source").unwrap_or_default(),
            update_time: row.try_get("update_time").unwrap_or_default(),
            signature: row.try_get("signature").unwrap_or_default(),
            create_time: row.try_get("create_time").unwrap_or_default(),
            email: row.try_get("email").unwrap_or_default(),
        })
    }
}

impl From<User> for FriendshipWithUser {
    fn from(value: User) -> Self {
        Self {
            user_id: value.id,
            name: value.name,
            account: value.account,
            avatar: value.avatar,
            gender: value.gender,
            age: value.age,
            region: value.region,
            ..Default::default()
        }
    }
}

impl FromRow<'_, PgRow> for FriendshipWithUser {
    fn from_row(row: &'_ PgRow) -> Result<Self, Error> {
        Ok(Self {
            fs_id: row.try_get("fs_id").unwrap_or_default(),
            user_id: row.try_get("user_id").unwrap_or_default(),
            name: row.try_get("name").unwrap_or_default(),
            account: row.try_get("account").unwrap_or_default(),
            avatar: row.try_get("avatar").unwrap_or_default(),
            gender: row.try_get("gender").unwrap_or_default(),
            age: row.try_get("age").unwrap_or_default(),
            region: row.try_get("region").unwrap_or_default(),
            status: row.try_get("status").unwrap_or_default(),
            apply_msg: row.try_get("apply_msg").unwrap_or_default(),
            source: row.try_get("source").unwrap_or_default(),
            create_time: row.try_get("create_time").unwrap_or_default(),
            email: row.try_get("email").unwrap_or_default(),
            remark: None,
        })
    }
}

impl From<User> for Friend {
    fn from(value: User) -> Self {
        Self {
            fs_id: String::new(),
            friend_id: value.id,
            name: value.name,
            account: value.account,
            avatar: value.avatar,
            gender: value.gender,
            age: value.age,
            region: value.region,
            status: 0,
            remark: None,
            source: "".to_string(),
            update_time: 0,
            signature: value.signature,
            create_time: 0,
            email: value.email,
        }
    }
}
