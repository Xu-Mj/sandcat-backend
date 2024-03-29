use std::fmt::Debug;

use async_trait::async_trait;

use abi::config::Config;
use abi::errors::Error;

mod redis;

#[async_trait]
pub trait Cache: Sync + Send + Debug {
    async fn get_seq(&self, user_id: &str) -> Result<i64, Error>;
    async fn query_group_members_id(&self, group_id: &str) -> Result<Option<Vec<String>>, Error>;
}

pub fn cache(config: &Config) -> Box<dyn Cache> {
    Box::new(redis::RedisCache::from_config(config))
}
