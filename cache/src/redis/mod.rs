use crate::Cache;
use abi::config::Config;
use abi::errors::Error;
use async_trait::async_trait;
use redis::AsyncCommands;

/// group members id prefix
const GROUP_MEMBERS_ID_PREFIX: &str = "group_members_id";

/// register code key
const REGISTER_CODE_KEY: &str = "register_code";

/// register code expire time
const REGISTER_CODE_EXPIRE: i64 = 300;

const USER_ONLINE_SET: &str = "user_online_set";

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
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        // increase seq
        let seq: i64 = conn.get(&key).await.unwrap_or_default();
        Ok(seq)
    }
    async fn increase_seq(&self, user_id: &str) -> Result<i64, Error> {
        // generate key
        let key = format!("seq:{}", user_id);

        // get seq from redis
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        // increase seq
        let seq: i64 = conn.incr(&key, 1).await?;
        Ok(seq)
    }

    /// the group members id in redis is a set, with group_members_id:group_id as key
    async fn query_group_members_id(&self, group_id: &str) -> Result<Vec<String>, Error> {
        // generate key
        let key = format!("{}:{}", GROUP_MEMBERS_ID_PREFIX, group_id);
        // query value from redis
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        let result: Vec<String> = conn.smembers(&key).await?;
        Ok(result)
    }

    async fn save_group_members_id(
        &self,
        group_id: &str,
        members_id: Vec<String>,
    ) -> Result<(), Error> {
        let key = format!("{}:{}", GROUP_MEMBERS_ID_PREFIX, group_id);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        // Add each member to the set for the group by redis pipe
        let mut pipe = redis::pipe();
        for member in members_id {
            pipe.sadd(&key, &member);
        }
        pipe.query_async(&mut conn).await?;
        Ok(())
    }

    async fn add_group_member_id(&self, member_id: &str, group_id: &str) -> Result<(), Error> {
        let key = format!("{}:{}", GROUP_MEMBERS_ID_PREFIX, group_id);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        conn.sadd(&key, member_id).await?;
        Ok(())
    }

    async fn remove_group_member_id(&self, group_id: &str, member_id: &str) -> Result<(), Error> {
        let key = format!("{}:{}", GROUP_MEMBERS_ID_PREFIX, group_id);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        conn.srem(&key, member_id).await?;
        Ok(())
    }

    async fn del_group_members(&self, group_id: &str) -> Result<(), Error> {
        let key = format!("{}:{}", GROUP_MEMBERS_ID_PREFIX, group_id);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        conn.del(&key).await?;
        Ok(())
    }

    async fn save_register_code(&self, email: &str, code: &str) -> Result<(), Error> {
        // set the register code with 5 minutes expiration time
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        // use pipe to exec two commands
        let mut pipe = redis::pipe();
        pipe.hset(REGISTER_CODE_KEY, email, code)
            .expire(REGISTER_CODE_KEY, REGISTER_CODE_EXPIRE)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    async fn get_register_code(&self, email: &str) -> Result<Option<String>, Error> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let result = conn.hget(REGISTER_CODE_KEY, email).await?;
        Ok(result)
    }

    async fn del_register_code(&self, email: &str) -> Result<(), Error> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        conn.hdel(REGISTER_CODE_KEY, email).await?;
        Ok(())
    }

    async fn user_login(&self, user_id: &str) -> Result<(), Error> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        conn.sadd(USER_ONLINE_SET, user_id).await?;
        Ok(())
    }

    async fn user_logout(&self, user_id: &str) -> Result<(), Error> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        conn.srem(USER_ONLINE_SET, user_id).await?;
        Ok(())
    }

    async fn online_count(&self) -> Result<i64, Error> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let result: i64 = conn.scard(USER_ONLINE_SET).await?;
        Ok(result)
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
            thread::spawn(move || {
                Runtime::new().unwrap().block_on(async {
                    let mut conn = client.get_multiplexed_async_connection().await.unwrap();
                    // let _: () to tell the compiler that the query_async method's return type is ()
                    let _: () = redis::cmd("FLUSHDB").query_async(&mut conn).await.unwrap();
                })
            })
            .join()
            .unwrap();
        }
    }
    impl TestRedis {
        fn new() -> Self {
            // use the 9 database for test
            let database = 9;
            Self::from_db(database)
        }

        // because of the tests are running in parallel,
        // we need to use different database,
        // in case of the flush db command will cause conflict in drop method
        fn from_db(db: u8) -> Self {
            let config = Config::load("../abi/fixtures/im.yml").unwrap();
            let url = format!("{}/{}", config.redis.url(), db);
            let client = redis::Client::open(url).unwrap();
            let cache = RedisCache::new(client.clone());
            TestRedis { client, cache }
        }
    }
    #[tokio::test]
    async fn test_increase_seq() {
        let user_id = "test";
        let cache = TestRedis::new();
        let seq = cache.increase_seq(user_id).await.unwrap();
        assert_eq!(seq, 1);
    }

    #[tokio::test]
    async fn test_save_group_members_id() {
        let group_id = "test";
        let members_id = vec!["1".to_string(), "2".to_string()];
        let cache = TestRedis::new();
        let result = cache.save_group_members_id(group_id, members_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_group_members_id() {
        let group_id = "test";
        let members_id = vec!["1".to_string(), "2".to_string()];
        let db = 8;
        let cache = TestRedis::from_db(db);
        let result = cache.save_group_members_id(group_id, members_id).await;
        assert!(result.is_ok());
        let result = cache.query_group_members_id(group_id).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"1".to_string()));
        assert!(result.contains(&"2".to_string()));
    }

    #[tokio::test]
    async fn test_add_group_member_id() {
        let group_id = "test";
        let member_id = "1";
        let cache = TestRedis::new();
        let result = cache.add_group_member_id(member_id, group_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_group_member_id() {
        let group_id = "test";
        let member_id = "1";
        let cache = TestRedis::new();
        let result = cache.add_group_member_id(member_id, group_id).await;
        assert!(result.is_ok());
        let result = cache.remove_group_member_id(group_id, member_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_del_group_members() {
        let group_id = "test";
        let members_id = vec!["1".to_string(), "2".to_string()];
        let cache = TestRedis::new();
        // need to add first
        let result = cache.save_group_members_id(group_id, members_id).await;
        assert!(result.is_ok());
        let result = cache.del_group_members(group_id).await;
        assert!(result.is_ok());
    }
}
