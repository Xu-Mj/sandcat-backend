use crate::message::{User, UserWithMatchType};
use sqlx::postgres::PgRow;
use sqlx::{Error, FromRow, Row};

impl FromRow<'_, PgRow> for User {
    fn from_row(row: &'_ PgRow) -> Result<Self, Error> {
        Ok(User {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            account: row.try_get("account")?,
            password: row.try_get("password")?,
            avatar: row.try_get("avatar")?,
            gender: row.try_get("gender")?,
            age: row.try_get("age")?,
            phone: row.try_get("phone")?,
            email: row.try_get("email")?,
            address: row.try_get("address")?,
            region: row.try_get("region")?,
            birthday: row.try_get("birthday")?,
            create_time: row.try_get("create_time")?,
            update_time: row.try_get("update_time")?,
        })
    }
}
impl FromRow<'_, PgRow> for UserWithMatchType {
    fn from_row(row: &'_ PgRow) -> Result<Self, Error> {
        Ok(UserWithMatchType {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            account: row.try_get("account")?,
            password: row.try_get("password")?,
            avatar: row.try_get("avatar")?,
            gender: row.try_get("gender")?,
            age: row.try_get("age")?,
            phone: row.try_get("phone")?,
            email: row.try_get("email")?,
            address: row.try_get("address")?,
            region: row.try_get("region")?,
            birthday: row.try_get("birthday")?,
            create_time: row.try_get("create_time")?,
            update_time: row.try_get("update_time")?,
            match_type: row.try_get("match_type")?,
        })
    }
}
