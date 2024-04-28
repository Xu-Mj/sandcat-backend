use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;

use abi::config::Config;
use abi::errors::Error;

mod redis;

#[async_trait]
pub trait Cache: Sync + Send + Debug {
    /// query sequence by user id
    async fn get_seq(&self, user_id: &str) -> Result<i64, Error>;
    async fn increase_seq(&self, user_id: &str) -> Result<i64, Error>;

    /// INCREASE GROUP MEMBERS SEQUENCE
    async fn incr_group_seq(&self, members: &[String]) -> Result<(), Error>;

    /// query group members id
    async fn query_group_members_id(&self, group_id: &str) -> Result<Vec<String>, Error>;

    /// save group members id, usually called when create group
    async fn save_group_members_id(
        &self,
        group_id: &str,
        members_id: Vec<String>,
    ) -> Result<(), Error>;

    /// add one member id to group members id set
    async fn add_group_member_id(&self, member_id: &str, group_id: &str) -> Result<(), Error>;

    /// remove the group member id from the group members id set
    async fn remove_group_member_id(&self, group_id: &str, member_id: &str) -> Result<(), Error>;

    /// return the members id
    async fn del_group_members(&self, group_id: &str) -> Result<(), Error>;

    /// save register code
    async fn save_register_code(&self, email: &str, code: &str) -> Result<(), Error>;

    /// get register code
    async fn get_register_code(&self, email: &str) -> Result<Option<String>, Error>;

    /// delete the register code after user register
    async fn del_register_code(&self, email: &str) -> Result<(), Error>;

    /// user login
    async fn user_login(&self, user_id: &str) -> Result<(), Error>;

    /// user logout
    async fn user_logout(&self, user_id: &str) -> Result<(), Error>;

    /// online count
    async fn online_count(&self) -> Result<i64, Error>;
}

pub fn cache(config: &Config) -> Arc<dyn Cache> {
    Arc::new(redis::RedisCache::from_config(config))
}
