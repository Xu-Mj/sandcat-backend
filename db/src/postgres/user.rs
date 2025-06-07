use argon2::{Argon2, PasswordHash, PasswordVerifier};
use async_trait::async_trait;
use sqlx::PgPool;

use abi::errors::{Error, Result};
use abi::message::{User, UserUpdate, UserWithMatchType};

use crate::user::UserRepo;

#[derive(Debug)]
pub struct PostgresUser {
    pool: PgPool,
    init_max_seq: i32,
}

impl PostgresUser {
    pub fn new(pool: PgPool, init_max_seq: i32) -> Self {
        PostgresUser { pool, init_max_seq }
    }
}

#[async_trait]
impl UserRepo for PostgresUser {
    async fn create_user(&self, user: User) -> Result<User> {
        let now = chrono::Utc::now().timestamp_millis();
        let mut tx = self.pool.begin().await?;

        let result = sqlx::query_as(
            "INSERT INTO users (
                id, name, account, password, avatar, gender, age, phone, email,
                address, birthday, region, salt, signature, create_time, update_time,
                last_login_time, last_login_ip, two_factor_enabled, account_status,
                status, last_active_time, status_message,
                privacy_settings, notification_settings, language,
                friend_requests_privacy, profile_visibility,
                theme, timezone, is_delete
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28,
                $29, $30 , $31
            ) RETURNING *",
        )
        .bind(&user.id)
        .bind(&user.name)
        .bind(&user.account)
        .bind(&user.password)
        .bind(&user.avatar)
        .bind(&user.gender)
        .bind(user.age)
        .bind(&user.phone)
        .bind(&user.email)
        .bind(&user.address)
        .bind(user.birthday)
        .bind(&user.region)
        .bind(&user.salt)
        .bind(&user.signature)
        .bind(now) // create_time
        .bind(now) // update_time
        .bind(now) // last_login_time (设为创建时间)
        .bind(&user.last_login_ip)
        .bind(user.two_factor_enabled)
        .bind(&user.account_status)
        .bind(&user.status)
        .bind(now) // last_active_time (设为创建时间)
        .bind(&user.status_message)
        .bind(&user.privacy_settings)
        .bind(&user.notification_settings)
        .bind(&user.language)
        .bind(&user.friend_requests_privacy)
        .bind(&user.profile_visibility)
        .bind(&user.theme)
        .bind(&user.timezone)
        .bind(false) // is_delete
        .fetch_one(&mut *tx)
        .await?;

        // insert into sequence (保持不变)
        sqlx::query(
            "INSERT INTO sequence (user_id, send_max_seq, rec_max_seq) VALUES ($1, $2, $2)",
        )
        .bind(&user.id)
        .bind(self.init_max_seq)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(result)
    }

    async fn get_user_by_id(&self, id: &str) -> Result<Option<User>> {
        let user = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(user)
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;
        Ok(user)
    }

    /// not allow to use username
    async fn search_user(&self, user_id: &str, pattern: &str) -> Result<Option<UserWithMatchType>> {
        let user = sqlx::query_as(
            "SELECT id, name, account, avatar, gender, age, email, region, birthday, signature,
             status, last_active_time, profile_visibility,
             CASE
                WHEN phone = $2 THEN 'phone'
                WHEN email = $2 THEN 'email'
                WHEN account = $2 THEN 'account'
                ELSE null
             END AS match_type
             FROM users WHERE id <> $1 AND (email = $2 OR phone = $2 OR account = $2) AND NOT is_delete"
        )
        .bind(user_id)
        .bind(pattern)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    async fn update_user(&self, user: UserUpdate) -> Result<User> {
        let user = sqlx::query_as(
            "UPDATE users SET
            name = COALESCE(NULLIF($2, ''), name),
            avatar = COALESCE(NULLIF($3, ''), avatar),
            gender = COALESCE(NULLIF($4, ''), gender),
            phone = COALESCE(NULLIF($5, ''), phone),
            email = COALESCE(NULLIF($6, ''), email),
            address = COALESCE(NULLIF($7, ''), address),
            region = COALESCE(NULLIF($8, ''), region),
            birthday = COALESCE(NULLIF($9, 0), birthday),
            signature = COALESCE(NULLIF($10, ''), signature),

            status = COALESCE(NULLIF($11, ''), status),
            status_message = COALESCE(NULLIF($12, ''), status_message),

            privacy_settings = COALESCE(NULLIF($13, '{}'), privacy_settings),
            notification_settings = COALESCE(NULLIF($14, '{}'), notification_settings),
            language = COALESCE(NULLIF($15, ''), language),

            friend_requests_privacy = COALESCE(NULLIF($16, ''), friend_requests_privacy),
            profile_visibility = COALESCE(NULLIF($17, ''), profile_visibility),

            theme = COALESCE(NULLIF($18, ''), theme),
            timezone = COALESCE(NULLIF($19, ''), timezone),

            update_time = $20
            WHERE id = $1
            RETURNING *",
        )
        .bind(&user.id)
        .bind(&user.name)
        .bind(&user.avatar)
        .bind(&user.gender)
        .bind(&user.phone)
        .bind(&user.email)
        .bind(&user.address)
        .bind(&user.region)
        .bind(user.birthday)
        .bind(&user.signature)
        .bind(&user.status)
        .bind(&user.status_message)
        .bind(&user.privacy_settings)
        .bind(&user.notification_settings)
        .bind(&user.language)
        .bind(&user.friend_requests_privacy)
        .bind(&user.profile_visibility)
        .bind(&user.theme)
        .bind(&user.timezone)
        .bind(chrono::Utc::now().timestamp_millis())
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    async fn update_region(&self, user_id: &str, region: &str) -> Result<()> {
        sqlx::query("UPDATE users SET region = $2 WHERE id = $1")
            .bind(user_id)
            .bind(region)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn verify_pwd(&self, account: &str, password: &str) -> Result<Option<User>> {
        let user: Option<User> =
            sqlx::query_as("SELECT * FROM users WHERE account = $1 OR phone = $1 OR email = $1")
                .bind(account)
                .fetch_optional(&self.pool)
                .await?;
        if user.is_none() {
            return Ok(None);
        }

        let mut user = user.unwrap();
        let parsed_hash = PasswordHash::new(&user.password)
            .map_err(|e| Error::internal_with_details(e.to_string()))?;

        let is_valid = Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok();
        user.password = "".to_string();
        if !is_valid {
            return Ok(None);
        }
        Ok(Some(user))
    }

    async fn modify_pwd(&self, user_id: &str, password: &str) -> Result<()> {
        let mut user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&__self.pool)
            .await?;
        if user.salt.is_empty() {
            user.salt = utils::generate_salt();
        }

        let password = utils::hash_password(password.as_bytes(), &user.salt)?;
        sqlx::query("UPDATE users SET salt = $2, password = $3 WHERE id = $1")
            .bind(user_id)
            .bind(&user.salt)
            .bind(password)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_user_status(
        &self,
        user_id: &str,
        status: &str,
        message: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        if let Some(msg) = message {
            sqlx::query("UPDATE users SET status = $2, status_message = $3, last_active_time = $4 WHERE id = $1")
                .bind(user_id)
                .bind(status)
                .bind(msg)
                .bind(now)
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("UPDATE users SET status = $2, last_active_time = $3 WHERE id = $1")
                .bind(user_id)
                .bind(status)
                .bind(now)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    async fn update_login_info(&self, user_id: &str, ip: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        sqlx::query("UPDATE users SET last_login_time = $2, last_login_ip = $3, login_attempts = 0 WHERE id = $1")
            .bind(user_id)
            .bind(now)
            .bind(ip)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
