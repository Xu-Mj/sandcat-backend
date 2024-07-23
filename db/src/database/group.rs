use async_trait::async_trait;

use abi::errors::Error;
use abi::message::{
    GetGroupAndMembersResp, GroupCreate, GroupInfo, GroupInvitation, GroupInviteNew, GroupMember,
    GroupUpdate,
};

#[async_trait]
pub trait GroupStoreRepo: Sync + Send {
    async fn get_group(&self, user_id: &str, group_id: &str) -> Result<GroupInfo, Error>;

    async fn get_group_and_members(
        &self,
        user_id: &str,
        group_id: &str,
    ) -> Result<GetGroupAndMembersResp, Error>;

    async fn get_members(
        &self,
        user_id: &str,
        group_id: &str,
        mem_ids: Vec<String>,
    ) -> Result<Vec<GroupMember>, Error>;

    async fn create_group_with_members(
        &self,
        group: &GroupCreate,
    ) -> Result<GroupInvitation, Error>;

    async fn invite_new_members(&self, group: &GroupInviteNew) -> Result<(), Error>;

    async fn remove_member(&self, group_id: &str, user_id: &str, mem_id: &str)
        -> Result<(), Error>;

    #[allow(dead_code)]
    async fn get_group_by_id(&self, group_id: &str) -> Result<GroupInfo, Error>;

    async fn query_group_members_id(&self, group_id: &str) -> Result<Vec<String>, Error>;

    #[allow(dead_code)]
    async fn query_group_members_by_group_id(
        &self,
        group_id: &str,
    ) -> Result<Vec<GroupMember>, Error>;

    async fn update_group(&self, group: &GroupUpdate) -> Result<GroupInfo, Error>;

    async fn exit_group(&self, user_id: &str, group_id: &str) -> Result<(), Error>;

    async fn delete_group(&self, group_id: &str, owner: &str) -> Result<GroupInfo, Error>;
}
