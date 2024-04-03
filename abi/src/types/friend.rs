use crate::message::{Friend, Friendship, FriendshipStatus, FriendshipWithUser, User};
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
            FriendshipStatus::Default => write!(f, "Default"),
            FriendshipStatus::Canceled => write!(f, "Canceled"),
            FriendshipStatus::Deleted => write!(f, "Deleted"),
        }
    }
}

impl FromRow<'_, PgRow> for Friendship {
    fn from_row(row: &'_ PgRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            friend_id: row.try_get("friend_id")?,
            status: row.try_get("status")?,
            apply_msg: row.try_get("apply_msg")?,
            req_remark: row.try_get("req_remark")?,
            resp_msg: row.try_get("resp_msg")?,
            resp_remark: row.try_get("resp_remark")?,
            source: row.try_get("source")?,
            create_time: row.try_get("create_time")?,
            accept_time: row.try_get("accept_time")?,
        })
    }
}

impl FromRow<'_, PgRow> for Friend {
    fn from_row(row: &'_ PgRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            account: row.try_get("account")?,
            avatar: row.try_get("avatar")?,
            gender: row.try_get("gender")?,
            age: row.try_get("age")?,
            region: row.try_get("region")?,
            status: row.try_get("status")?,
            hello: row.try_get("hello")?,
            remark: row.try_get("remark")?,
            source: row.try_get("source")?,
            accept_time: row.try_get("accept_time")?,
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
            fs_id: row.try_get("fs_id")?,
            user_id: row.try_get("user_id")?,
            name: row.try_get("name")?,
            account: row.try_get("account")?,
            avatar: row.try_get("avatar")?,
            gender: row.try_get("gender")?,
            age: row.try_get("age")?,
            region: row.try_get("region")?,
            status: row.try_get("status")?,
            apply_msg: row.try_get("apply_msg")?,
            source: row.try_get("source")?,
            create_time: row.try_get("create_time")?,
        })
    }
}

impl From<User> for Friend {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            name: value.name,
            account: value.account,
            avatar: value.avatar,
            gender: value.gender,
            age: value.age,
            region: value.region,
            status: 0,
            hello: None,
            remark: None,
            source: "".to_string(),
            accept_time: 0,
        }
    }
}
