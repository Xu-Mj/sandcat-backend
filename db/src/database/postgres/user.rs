use argon2::{Argon2, PasswordHash, PasswordVerifier};
use async_trait::async_trait;
use sqlx::PgPool;

use abi::errors::Error;
use abi::message::{User, UserUpdate, UserWithMatchType};

use crate::database::user::UserRepo;

#[derive(Debug)]
pub struct PostgresUser {
    pool: PgPool,
}

impl PostgresUser {
    pub fn new(pool: PgPool) -> Self {
        PostgresUser { pool }
    }
}

#[async_trait]
impl UserRepo for PostgresUser {
    async fn create_user(&self, user: User) -> Result<User, Error> {
        let now = chrono::Local::now().timestamp_millis();
        let result = sqlx::query_as(
            "INSERT INTO users
            (id, name, account, password, avatar, gender, age, phone, email, address, region, salt, signature, create_time, update_time)
            VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) RETURNING *")
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
            .bind(&user.region)
            .bind(&user.salt)
            .bind(&user.signature)
            .bind(now)
            .bind(now)
            .fetch_one(&self.pool)
            .await?;
        Ok(result)
    }

    async fn get_user_by_id(&self, id: &str) -> Result<Option<User>, Error> {
        let user = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(user)
    }

    /// not allow to use username
    async fn search_user(
        &self,
        user_id: &str,
        pattern: &str,
    ) -> Result<Option<UserWithMatchType>, Error> {
        let user = sqlx::query_as(
            "SELECT id, name, account, avatar, gender, age, email, region, birthday, signature,
             CASE
                WHEN phone = $2 THEN 'phone'
                WHEN email = $2 THEN 'email'
                WHEN account = $2 THEN 'account'
                ELSE null
             END AS match_type
             FROM users WHERE id <> $1 AND (email = $2 OR phone = $2 OR account = $2)",
        )
        .bind(user_id)
        .bind(pattern)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    async fn update_user(&self, user: UserUpdate) -> Result<User, Error> {
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
            update_time = $11
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
        .bind(chrono::Local::now().timestamp_millis())
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    async fn verify_pwd(&self, account: &str, password: &str) -> Result<Option<User>, Error> {
        let user: Option<User> =
            sqlx::query_as("SELECT * FROM users WHERE account = $1 OR phone = $1 OR email = $1")
                .bind(account)
                .fetch_optional(&self.pool)
                .await?;
        if user.is_none() {
            return Ok(None);
        }

        let mut user = user.unwrap();
        let parsed_hash =
            PasswordHash::new(&user.password).map_err(|e| Error::InternalServer(e.to_string()))?;
        let is_valid = Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok();
        user.password = "".to_string();
        if !is_valid {
            return Ok(None);
        }
        Ok(Some(user))
    }
}
