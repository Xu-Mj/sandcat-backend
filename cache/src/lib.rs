use std::fmt::Debug;

use async_trait::async_trait;

use abi::config::Config;
use abi::errors::Error;

mod redis;

#[async_trait]
pub trait Cache: Sync + Send + Debug {
    async fn get_seq(&self, user_id: &str) -> Result<i64, Error>;

    async fn query_group_members_id(&self, group_id: &str) -> Result<Vec<String>, Error>;

    async fn save_group_members_id(
        &self,
        group_id: &str,
        members_id: Vec<String>,
    ) -> Result<(), Error>;

    async fn add_group_member_id(&self, member_id: &str, group_id: &str) -> Result<(), Error>;

    async fn remove_group_member_id(&self, group_id: &str, member_id: &str) -> Result<(), Error>;

    // return the members id
    async fn del_group_members(&self, group_id: &str) -> Result<(), Error>;
}

pub fn cache(config: &Config) -> Box<dyn Cache> {
    Box::new(redis::RedisCache::from_config(config))
}
