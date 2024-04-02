use async_trait::async_trait;
use tokio::sync::mpsc;

use abi::errors::Error;
use abi::message::{Friendship, FriendshipStatus, FsCreate, FsUpdate};

#[async_trait]
pub trait FriendRepo: Send + Sync {
    async fn create_fs(&self, fs: FsCreate) -> Result<Friendship, Error>;

    async fn get_fs(&self, user_id: &str, friend_id: &str) -> Result<Friendship, Error>;

    async fn get_friend_list(&self, user_id: &str) -> Result<mpsc::Receiver<Friendship>, Error>;

    /// update friend apply request
    async fn update_fs(&self, fs: FsUpdate) -> Result<Friendship, Error>;

    async fn update_friend_remark(
        &self,
        user_id: &str,
        friend_id: &str,
        remark: &str,
    ) -> Result<Friendship, Error>;

    async fn update_friend_status(
        &self,
        user_id: &str,
        friend_id: &str,
        status: FriendshipStatus,
    ) -> Result<Friendship, Error>;
}
