use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "friend_request_status")]
pub enum FriendStatus {
    #[default]
    Pending,
    Accepted,
    Rejected,
    Blacked,
    Cancelled,
}

/*rpc FromSql<FriendRequestStatus, Pg> for FriendStatus {
    fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let status = <String as FromSql<diesel::sql_types::Text, Pg>>::from_sql(bytes)?;
        match status.as_str() {
            "Pending" => Ok(FriendStatus::Pending),
            "Accepted" => Ok(FriendStatus::Accepted),
            "Rejected" => Ok(FriendStatus::Rejected),
            "Blacked" => Ok(FriendStatus::Blacked),
            "Cancelled" => Ok(FriendStatus::Cancelled),
            _ => Err(format!("Invalid friend status: {}", &status,).into()),
        }
    }
}*/

/*rpc ToSql<FriendRequestStatus, Pg> for FriendStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
        match self {
            FriendStatus::Pending => out.write_all(b"Pending")?,
            FriendStatus::Accepted => out.write_all(b"Accepted")?,
            FriendStatus::Rejected => out.write_all(b"Rejected")?,
            FriendStatus::Blacked => out.write_all(b"Blacked")?,
            FriendStatus::Cancelled => out.write_all(b"Cancelled")?,
        }
        Ok(diesel::serialize::IsNull::No)
    }
}
*/
/*rpc Queryable<FriendRequestStatus, Pg> for FriendStatus {
    type Row = Self;

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(row)
    }
}
*/
impl Display for FriendStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FriendStatus::Pending => f.write_str("Pending"),
            FriendStatus::Accepted => f.write_str("Accepted"),
            FriendStatus::Rejected => f.write_str("Rejected"),
            FriendStatus::Blacked => f.write_str("Blacked"),
            FriendStatus::Cancelled => f.write_str("Cancelled"),
        }
    }
}
