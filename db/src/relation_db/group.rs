use async_trait::async_trait;

use abi::errors::Error;
use abi::message::{GroupCreate, GroupInfo, GroupInvitation};

#[async_trait]
pub trait GroupStoreRepo: Sync + Send {
    async fn create_group_with_members(&self, group: GroupCreate)
        -> Result<GroupInvitation, Error>;

    async fn get_group_by_id(&self, group_id: &str) -> Result<GroupInfo, Error>;
}
