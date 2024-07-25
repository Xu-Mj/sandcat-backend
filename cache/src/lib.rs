use std::fmt::Debug;
use std::sync::Arc;

use abi::message::GroupMemSeq;
use async_trait::async_trait;

use abi::config::Config;
use abi::errors::Error;

mod redis;

#[async_trait]
pub trait Cache: Sync + Send + Debug {
    /// check if the sequence is loaded
    async fn check_seq_loaded(&self) -> Result<bool, Error>;

    /// set the sequence is loaded
    async fn set_seq_loaded(&self) -> Result<(), Error>;

    /// set the receive sequence
    /// contains: user_id, send_max_seq, recv_max_seq
    async fn set_seq(&self, max_seq: &[(String, i64, i64)]) -> Result<(), Error>;

    /// set the send sequence
    async fn set_send_seq(&self, max_seq: &[(String, i64)]) -> Result<(), Error>;

    /// query receive sequence by user id
    async fn get_seq(&self, user_id: &str) -> Result<i64, Error>;
    /// query current send sequence and receive sequence by user id
    async fn get_cur_seq(&self, user_id: &str) -> Result<(i64, i64), Error>;

    /// query send sequence by user id,
    /// it returns the current send sequence and the max send sequence
    async fn get_send_seq(&self, user_id: &str) -> Result<(i64, i64), Error>;

    /// increase receive sequence by user id
    async fn increase_seq(&self, user_id: &str) -> Result<(i64, i64, bool), Error>;

    /// increase send sequence by user id
    async fn incr_send_seq(&self, user_id: &str) -> Result<(i64, i64, bool), Error>;

    /// INCREASE GROUP MEMBERS SEQUENCE
    async fn incr_group_seq(&self, mut members: Vec<String>) -> Result<Vec<GroupMemSeq>, Error>;

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

    async fn remove_group_member_batch(
        &self,
        group_id: &str,
        member_id: &[&str],
    ) -> Result<(), Error>;

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
