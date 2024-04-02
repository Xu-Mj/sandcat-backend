use crate::database::user::UserRepo;
use abi::errors::Error;
use abi::message::{User, UserWithMatchType};
use async_trait::async_trait;
use sqlx::PgPool;

#[derive(Debug)]
pub struct PostgresUser {
    pool: PgPool,
}

impl PostgresUser {
    pub async fn new(pool: PgPool) -> Self {
        PostgresUser { pool }
    }
}

#[async_trait]
impl UserRepo for PostgresUser {
    async fn create_user(&self, user: User) -> Result<User, Error> {
        let result = sqlx::query_as(
            "INSERT INTO users
            (id, name, account, password, avatar, gender, age, phone, email, address, region, birthday)
            VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING *")
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
            .bind(user.birthday)
            .fetch_one(&self.pool)
            .await?;
        Ok(result)
    }

    async fn get_user_by_id(&self, id: &str) -> Result<User, Error> {
        let user = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(user)
    }

    async fn search_user(
        &self,
        user_id: &str,
        pattern: &str,
    ) -> Result<Vec<UserWithMatchType>, Error> {
        let users: Vec<UserWithMatchType> = sqlx::query_as(
            "SELECT *,
             CASE
                WHEN name LIKE '%$2%' THEN 'name'
                WHEN phone = $2 THEN 'phone'
                ELSE null
             END AS match_type
             FROM users WHERE id <> $1 AND (name LIKE '%$2%' OR phone = $2)",
        )
        .bind(user_id)
        .bind(pattern)
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    async fn update_user(&self, user: User) -> Result<User, Error> {
        let user = sqlx::query_as(
            "UPDATE users SET
            name = COALESCE(NULLIF($2, ''), name),
            avatar = COALESCE(NULLIF($3, ''), avatar),
            gender = COALESCE(NULLIF($4, ''), gender),
            age = COALESCE(NULLIF($5, 0), age),
            phone = COALESCE(NULLIF($6, ''), phone),
            email = COALESCE(NULLIF($7, ''), email),
            address = COALESCE(NULLIF($8, ''), address),
            region = COALESCE(NULLIF($9, ''), region),
            birthday = COALESCE(NULLIF($10, 0), birthday)
            WHERE id = $1",
        )
        .bind(&user.id)
        .bind(&user.name)
        .bind(&user.avatar)
        .bind(&user.gender)
        .bind(user.age)
        .bind(&user.phone)
        .bind(&user.email)
        .bind(&user.address)
        .bind(&user.region)
        .bind(user.birthday)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }
}
