use std::fmt::Debug;

use async_trait::async_trait;

use abi::errors::Result;
use abi::message::{
    AgreeReply, Friend, FriendDb, FriendGroup, FriendPrivacySettings, FriendTag, Friendship,
    FriendshipStatus, FriendshipWithUser, FsCreate, FsUpdate,
};

#[async_trait]
pub trait FriendRepo: Send + Sync + Debug {
    /// create friend apply request, ignore friendship status in fs, it always be pending
    async fn create_fs(&self, fs: FsCreate) -> Result<(FriendshipWithUser, FriendshipWithUser)>;

    // is it necessary to exists?
    // async fn get_fs(&self, user_id: &str, friend_id: &str) -> Result<FriendshipWithUser, Error>;

    /// get friend apply request list
    async fn get_fs_list(
        &self,
        user_id: &str,
        offline_time: i64,
    ) -> Result<Vec<FriendshipWithUser>>;

    /// update friend apply request
    #[allow(dead_code)]
    async fn update_fs(&self, fs: FsUpdate) -> Result<Friendship>;

    /// update friend remark; the status should be accepted
    async fn update_friend_remark(
        &self,
        user_id: &str,
        friend_id: &str,
        remark: &str,
    ) -> Result<FriendDb>;

    /// update friend status; the status should be accepted or blocked.
    /// this is not that to agree friend-apply-request
    #[allow(dead_code)]
    async fn update_friend_status(
        &self,
        user_id: &str,
        friend_id: &str,
        status: FriendshipStatus,
    ) -> Result<Friendship>;

    async fn get_friend_list(&self, user_id: &str, offline_time: i64) -> Result<Vec<Friend>>;
    // ) -> Result<mpsc::Receiver<Result<Friend, Error>>, Error>;

    /// agree friend-apply-request
    async fn agree_friend_apply_request(&self, fs: AgreeReply) -> Result<(Friend, Friend)>;

    async fn delete_friend(&self, fs_id: &str, user_id: &str) -> Result<()>;

    // Friend Group Management
    async fn create_friend_group(
        &self,
        user_id: &str,
        name: &str,
        display_order: i32,
    ) -> Result<FriendGroup>;
    async fn update_friend_group(
        &self,
        group_id: &str,
        name: &str,
        display_order: i32,
    ) -> Result<FriendGroup>;
    async fn delete_friend_group(&self, group_id: &str) -> Result<()>;
    async fn get_friend_groups(&self, user_id: &str) -> Result<Vec<FriendGroup>>;
    async fn assign_friend_to_group(
        &self,
        user_id: &str,
        friend_id: &str,
        group_id: &str,
    ) -> Result<FriendDb>;

    // Friend Tags Management
    async fn create_friend_tag(
        &self,
        user_id: &str,
        tag_name: &str,
        tag_color: &str,
    ) -> Result<FriendTag>;
    async fn delete_friend_tag(&self, tag_id: &str) -> Result<()>;
    async fn get_friend_tags(&self, user_id: &str) -> Result<Vec<FriendTag>>;
    async fn add_tags_to_friend(
        &self,
        user_id: &str,
        friend_id: &str,
        tag_ids: &[String],
    ) -> Result<()>;
    async fn remove_tags_from_friend(
        &self,
        user_id: &str,
        friend_id: &str,
        tag_ids: &[String],
    ) -> Result<()>;

    // Privacy Settings Management
    async fn update_friend_privacy(
        &self,
        user_id: &str,
        friend_id: &str,
        privacy_settings: &FriendPrivacySettings,
    ) -> Result<()>;
    async fn get_friend_privacy(
        &self,
        user_id: &str,
        friend_id: &str,
    ) -> Result<FriendPrivacySettings>;

    // Interaction Tracking
    async fn update_interaction_score(
        &self,
        user_id: &str,
        friend_id: &str,
        interaction_value: f32,
    ) -> Result<f32>;
    async fn get_interaction_stats(
        &self,
        user_id: &str,
        friend_id: &str,
    ) -> Result<(f32, i32, i64)>; // Returns (score, count, last_interaction)
}
