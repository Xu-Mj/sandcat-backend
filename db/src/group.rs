use std::fmt::Debug;

use async_trait::async_trait;

use abi::errors::Result;
use abi::message::{
    CreateAnnouncementRequest, CreateGroupCategoryRequest, CreatePollRequest,
    GetGroupAndMembersResp, GroupAnnouncement, GroupCategory, GroupCreate, GroupFile,
    GroupFileUploadRequest, GroupInfo, GroupInvitation, GroupInviteNew, GroupMember,
    GroupMuteRecord, GroupPoll, GroupUpdate, MuteGroupMemberRequest, UpdateGroupRequest,
    VotePollRequest,
};

#[async_trait]
pub trait GroupStoreRepo: Sync + Send + Debug {
    // 现有方法
    async fn get_group(&self, user_id: &str, group_id: &str) -> Result<GroupInfo>;
    async fn get_group_and_members(
        &self,
        user_id: &str,
        group_id: &str,
    ) -> Result<GetGroupAndMembersResp>;
    async fn get_members(
        &self,
        user_id: &str,
        group_id: &str,
        mem_ids: Vec<String>,
    ) -> Result<Vec<GroupMember>>;
    async fn create_group_with_members(&self, group: &GroupCreate) -> Result<GroupInvitation>;
    async fn invite_new_members(&self, group: &GroupInviteNew) -> Result<()>;
    async fn remove_member(&self, group_id: &str, user_id: &str, mem_ids: &[String]) -> Result<()>;
    async fn get_group_by_id(&self, group_id: &str) -> Result<GroupInfo>;
    async fn query_group_members_id(&self, group_id: &str) -> Result<Vec<String>>;
    async fn query_group_members_by_group_id(&self, group_id: &str) -> Result<Vec<GroupMember>>;
    async fn update_group(&self, group: &GroupUpdate) -> Result<GroupInfo>;
    async fn exit_group(&self, user_id: &str, group_id: &str) -> Result<()>;
    async fn delete_group(&self, group_id: &str, owner: &str) -> Result<GroupInfo>;

    // 扩展方法 - 增强群设置
    async fn update_group_settings(&self, req: &UpdateGroupRequest) -> Result<GroupInfo>;
    async fn update_member_settings(
        &self,
        group_id: &str,
        user_id: &str,
        notification_level: &str,
        display_order: i32,
    ) -> Result<GroupMember>;

    // 群组分类管理
    async fn create_group_category(
        &self,
        req: &CreateGroupCategoryRequest,
    ) -> Result<GroupCategory>;
    async fn update_group_category(&self, category: &GroupCategory) -> Result<GroupCategory>;
    async fn delete_group_category(&self, id: &str) -> Result<()>;
    async fn get_group_categories(&self) -> Result<Vec<GroupCategory>>;

    // 群组文件管理
    async fn upload_group_file(&self, req: &GroupFileUploadRequest) -> Result<GroupFile>;
    async fn delete_group_file(&self, file_id: &str, user_id: &str) -> Result<()>;
    async fn get_group_files(&self, group_id: &str) -> Result<Vec<GroupFile>>;
    async fn update_file_pin_status(&self, file_id: &str, is_pinned: bool) -> Result<GroupFile>;

    // 群组投票管理
    async fn create_poll(&self, req: &CreatePollRequest) -> Result<GroupPoll>;
    async fn vote_poll(&self, req: &VotePollRequest) -> Result<GroupPoll>;
    async fn close_poll(&self, poll_id: &str, user_id: &str) -> Result<GroupPoll>;
    async fn get_group_polls(&self, group_id: &str) -> Result<Vec<GroupPoll>>;
    async fn get_poll_details(&self, poll_id: &str) -> Result<GroupPoll>;

    // 群组禁言管理
    async fn mute_group_member(&self, req: &MuteGroupMemberRequest) -> Result<GroupMuteRecord>;
    async fn unmute_group_member(
        &self,
        group_id: &str,
        user_id: &str,
        operator_id: &str,
    ) -> Result<()>;
    async fn get_group_muted_members(&self, group_id: &str) -> Result<Vec<GroupMuteRecord>>;

    // 群组公告管理
    async fn create_announcement(
        &self,
        req: &CreateAnnouncementRequest,
    ) -> Result<GroupAnnouncement>;
    async fn delete_announcement(&self, id: &str, user_id: &str) -> Result<()>;
    async fn get_group_announcements(&self, group_id: &str) -> Result<Vec<GroupAnnouncement>>;
    async fn pin_announcement(&self, id: &str, is_pinned: bool) -> Result<GroupAnnouncement>;

    // 成员管理增强
    async fn change_member_role(
        &self,
        group_id: &str,
        user_id: &str,
        target_id: &str,
        new_role: &str,
    ) -> Result<GroupMember>;
}
