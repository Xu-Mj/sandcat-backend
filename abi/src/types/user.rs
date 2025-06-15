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
            salt: row.try_get("salt")?,
            signature: row.try_get("signature")?,

            // 新增字段
            last_login_time: row.try_get("last_login_time")?,
            last_login_ip: row.try_get("last_login_ip")?,
            two_factor_enabled: row.try_get("two_factor_enabled")?,
            account_status: row.try_get("account_status")?,

            status: row.try_get("status")?,
            last_active_time: row.try_get("last_active_time")?,
            status_message: row.try_get("status_message")?,

            privacy_settings: row.try_get("privacy_settings")?,
            notification_settings: row.try_get("notification_settings")?,
            language: row.try_get("language")?,

            friend_requests_privacy: row.try_get("friend_requests_privacy")?,
            profile_visibility: row.try_get("profile_visibility")?,

            theme: row.try_get("theme")?,
            timezone: row.try_get("timezone")?,

            is_delete: row.try_get("is_delete")?,
        })
    }
}
impl FromRow<'_, PgRow> for UserWithMatchType {
    fn from_row(row: &'_ PgRow) -> Result<Self, Error> {
        Ok(UserWithMatchType {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            account: row.try_get("account")?,
            avatar: row.try_get("avatar")?,
            gender: row.try_get("gender")?,
            age: row.try_get("age")?,
            email: row.try_get("email")?,
            region: row.try_get("region")?,
            birthday: row.try_get("birthday")?,
            match_type: row.try_get("match_type")?,
            signature: row.try_get("signature")?,
            is_friend: false,
        })
    }
}
