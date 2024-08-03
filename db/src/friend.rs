use std::fmt::Debug;

use async_trait::async_trait;

use abi::errors::Error;
use abi::message::{
    AgreeReply, Friend, FriendDb, Friendship, FriendshipStatus, FriendshipWithUser, FsCreate,
    FsUpdate,
};

#[async_trait]
pub trait FriendRepo: Send + Sync + Debug {
    /// create friend apply request, ignore friendship status in fs, it always be pending
    async fn create_fs(
        &self,
        fs: FsCreate,
    ) -> Result<(FriendshipWithUser, FriendshipWithUser), Error>;

    /// is it necessary to exists?
    // async fn get_fs(&self, user_id: &str, friend_id: &str) -> Result<FriendshipWithUser, Error>;

    /// get friend apply request list
    async fn get_fs_list(
        &self,
        user_id: &str,
        offline_time: i64,
    ) -> Result<Vec<FriendshipWithUser>, Error>;

    /// update friend apply request
    #[allow(dead_code)]
    async fn update_fs(&self, fs: FsUpdate) -> Result<Friendship, Error>;

    /// update friend remark; the status should be accepted
    async fn update_friend_remark(
        &self,
        user_id: &str,
        friend_id: &str,
        remark: &str,
    ) -> Result<FriendDb, Error>;

    /// update friend status; the status should be accepted or blocked.
    /// this is not that to agree friend-apply-request
    #[allow(dead_code)]
    async fn update_friend_status(
        &self,
        user_id: &str,
        friend_id: &str,
        status: FriendshipStatus,
    ) -> Result<Friendship, Error>;

    async fn get_friend_list(&self, user_id: &str, offline_time: i64)
        -> Result<Vec<Friend>, Error>;
    // ) -> Result<mpsc::Receiver<Result<Friend, Error>>, Error>;

    /// agree friend-apply-request
    async fn agree_friend_apply_request(&self, fs: AgreeReply) -> Result<(Friend, Friend), Error>;

    async fn delete_friend(&self, fs_id: &str, user_id: &str) -> Result<(), Error>;
}
