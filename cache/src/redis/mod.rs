use crate::Cache;
use abi::config::Config;
use abi::errors::Error;
use async_trait::async_trait;
use redis::AsyncCommands;

const GROUP_MEMBERS_ID_PREFIX: &str = "group_members_id";
#[derive(Debug)]
pub struct RedisCache {
    client: redis::Client,
}

impl RedisCache {
    #[allow(dead_code)]
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }
    pub fn from_config(config: &Config) -> Self {
        let client = redis::Client::open(config.redis.url()).unwrap();
        RedisCache { client }
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get_seq(&self, user_id: &str) -> Result<i64, Error> {
        // generate key
        let key = format!("seq:{}", user_id);

        // get seq from redis
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .unwrap();

        // increase seq
        let seq: i64 = conn.incr(&key, 1).await?;
        Ok(seq)
    }

    async fn query_group_members_id(&self, group_id: &str) -> Result<Option<Vec<String>>, Error> {
        // generate key
        let key = format!("{}:{}", GROUP_MEMBERS_ID_PREFIX, group_id);
        // query value from redis
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .unwrap();

        let result: Vec<String> = conn.smembers(&key).await?;
        // return None if result is empty, this means that this need to query from db
        if result.is_empty() {
            return Ok(None);
        }
        Ok(Some(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abi::config::Config;
    use std::ops::Deref;
    use std::thread;
    use tokio::runtime::Runtime;

    struct TestRedis {
        client: redis::Client,
        key: String,
        cache: RedisCache,
    }

    impl Deref for TestRedis {
        type Target = RedisCache;
        fn deref(&self) -> &Self::Target {
            &self.cache
        }
    }

    impl Drop for TestRedis {
        fn drop(&mut self) {
            let client = self.client.clone();
            let key = self.key.clone();
            thread::spawn(move || {
                Runtime::new().unwrap().block_on(async {
                    let mut conn = client.get_multiplexed_async_connection().await.unwrap();
                    let _: i64 = conn.del(&key).await.unwrap();
                })
            })
            .join()
            .unwrap();
        }
    }
    impl TestRedis {
        fn new(config: &Config, user_id: &str) -> Self {
            let client = redis::Client::open(config.redis.url()).unwrap();
            let key = format!("seq:{}", user_id);
            let cache = RedisCache::new(client.clone());
            TestRedis { client, key, cache }
        }
    }
    #[tokio::test]
    async fn test_get_seq() {
        let config = Config::load("../abi/fixtures/im.yml").unwrap();
        let user_id = "test";
        let cache = TestRedis::new(&config, user_id);
        let seq = cache.get_seq(user_id).await.unwrap();
        assert_eq!(seq, 1);
    }
}
