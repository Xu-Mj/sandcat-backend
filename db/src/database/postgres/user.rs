use crate::database::user::UserRepo;
use abi::errors::Error;
use abi::message::User;
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
        let result = sqlx::query_as("INSERT INTO users (id, name, account, password, avatar, gender, age, phone, email, address, region, birthday) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING *").bind(&user.id)
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
        _user_id: &str,
        _pattern: &str,
    ) -> Result<Vec<(User, String)>, Error> {
        todo!()
    }

    async fn update_user(&self, _user: User) -> Result<User, Error> {
        todo!()
    }
}
