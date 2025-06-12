use std::fmt::Debug;

use async_trait::async_trait;

use abi::errors::Result;
use abi::message::{User, UserUpdate, UserWithMatchType};

#[async_trait]
pub trait UserRepo: Sync + Send + Debug + Debug {
    /// create user
    async fn create_user(&self, user: User) -> Result<User>;

    /// get user by id
    async fn get_user_by_id(&self, id: &str) -> Result<Option<User>>;

    /// get user by email
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;

    /// search user by pattern, return users and matched method
    async fn search_user(&self, user_id: &str, pattern: &str) -> Result<Option<UserWithMatchType>>;

    async fn update_user(&self, user: UserUpdate) -> Result<User>;

    /// update user region by user id
    async fn update_region(&self, user_id: &str, region: &str) -> Result<()>;

    async fn verify_pwd(&self, account: &str, password: &str) -> Result<Option<User>>;

    async fn modify_pwd(&self, user_id: &str, password: &str) -> Result<()>;

    async fn update_user_status(
        &self,
        user_id: &str,
        status: &str,
        message: Option<&str>,
    ) -> Result<()>;

    async fn update_login_info(&self, user_id: &str, ip: &str) -> Result<()>;
}
