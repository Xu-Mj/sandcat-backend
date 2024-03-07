use std::fmt::{Display, Formatter};
use std::io::Write;

use diesel::backend::Backend;
use diesel::deserialize::FromSql;
use diesel::pg::Pg;
use diesel::serialize::{Output, ToSql};
use diesel::{AsExpression, Queryable};
use serde::{Deserialize, Serialize};

use crate::infra::db::schema::sql_types::FriendRequestStatus;

#[derive(Debug, Default, Clone, Deserialize, Serialize, AsExpression)]
#[diesel(sql_type = crate::infra::db::schema::sql_types::FriendRequestStatus)]
pub enum FriendStatus {
    #[default]
    Pending,
    Accepted,
    Rejected,
    Blacked,
    Cancelled,
}

impl FromSql<FriendRequestStatus, Pg> for FriendStatus {
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
}

// impl FromSqlRow<FriendRequestStatus, Pg> for FriendStatus {
//     fn build_from_row<'a>(row: &impl Row<'a, Pg>) -> diesel::deserialize::Result<Self> {
//         Self::from_sql(row.take())
//     }
// }

impl ToSql<FriendRequestStatus, Pg> for FriendStatus {
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
impl Queryable<FriendRequestStatus, Pg> for FriendStatus {
    type Row = Self;

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(row)
    }
}
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
