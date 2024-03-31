use async_trait::async_trait;

use abi::errors::Error;
use abi::message::{GroupCreate, GroupInfo, GroupInvitation, GroupMember};

#[async_trait]
pub trait GroupStoreRepo: Sync + Send {
    async fn create_group_with_members(&self, group: GroupCreate)
        -> Result<GroupInvitation, Error>;

    async fn get_group_by_id(&self, group_id: &str) -> Result<GroupInfo, Error>;

    async fn query_group_members_id(&self, group_id: &str) -> Result<Vec<String>, Error>;

    async fn query_group_members_by_group_id(
        &self,
        group_id: &str,
    ) -> Result<Vec<GroupMember>, Error>;

    async fn update_group(&self, group: &GroupInfo) -> Result<GroupInfo, Error>;

    async fn exit_group(&self, user_id: &str, group_id: &str) -> Result<(), Error>;

    async fn delete_group(&self, group_id: &str, owner: &str) -> Result<GroupInfo, Error>;
}
