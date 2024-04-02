use crate::message::{Friendship, FriendshipStatus};
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
            update_time: row.try_get("update_time")?,
        })
    }
}
