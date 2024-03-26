use abi::config::Config;
use abi::errors::Error;
use async_trait::async_trait;

mod redis;

#[async_trait]
pub trait Cache: Sync + Send {
    async fn get_seq(&self, user_id: String) -> Result<i64, Error>;
}

pub fn cache(config: &Config) -> Box<dyn Cache> {
    Box::new(redis::RedisCache::from_config(config))
}
