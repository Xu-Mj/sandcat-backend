use async_trait::async_trait;
use tokio::sync::mpsc;

use abi::errors::Error;
use abi::message::{Friend, Friendship, FriendshipStatus, FsCreate, FsReply, FsUpdate};

#[async_trait]
pub trait FriendRepo: Send + Sync {
    /// create friend apply request, ignore friendship status in fs, it always be pending
    async fn create_fs(&self, fs: FsCreate) -> Result<Friendship, Error>;

    /// is it necessary to exists?
    async fn get_fs(&self, user_id: &str, friend_id: &str) -> Result<Friendship, Error>;

    /// get friend apply request list
    async fn get_fs_list(&self, user_id: &str) -> Result<Vec<Friendship>, Error>;

    /// update friend apply request
    async fn update_fs(&self, fs: FsUpdate) -> Result<Friendship, Error>;

    /// update friend remark; the status should be accepted
    async fn update_friend_remark(
        &self,
        user_id: &str,
        friend_id: &str,
        remark: &str,
    ) -> Result<Friendship, Error>;

    /// update friend status; the status should be accepted or blocked.
    /// this is not that to agree friend-apply-request
    async fn update_friend_status(
        &self,
        user_id: &str,
        friend_id: &str,
        status: FriendshipStatus,
    ) -> Result<Friendship, Error>;

    /// get friend list;
    /// we need to determine user_id is the friend or not
    /// use 'OR'
    async fn get_friend_list(
        &self,
        user_id: &str,
    ) -> Result<mpsc::Receiver<Result<Friend, Error>>, Error>;

    /// agree friend-apply-request
    async fn agree_friend_apply_request(&self, fs: FsReply) -> Result<Friendship, Error>;
}
