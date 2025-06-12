use axum::Json;
use axum::extract::State;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

use abi::errors::Error;
use abi::message::{
    CreateAnnouncementRequest, CreateGroupCategoryRequest, CreatePollRequest, GetMemberReq,
    GroupAnnouncement, GroupCategory, GroupCreate, GroupFile, GroupFileUploadRequest, GroupInfo,
    GroupInvitation, GroupInviteNew, GroupMember, GroupMuteRecord, GroupPoll, GroupUpdate, MsgType,
    MuteGroupMemberRequest, RemoveMemberRequest, SendMsgRequest, UpdateGroupRequest,
    VotePollRequest,
};

use crate::AppState;
use crate::api_utils::custom_extract::{JsonWithAuthExtractor, PathWithAuthExtractor};

/// get group information
/// need user id and group id
/// if user is not in the group, return error
pub async fn get_group(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, group_id)): PathWithAuthExtractor<(String, String)>,
) -> Result<Json<GroupInfo>, Error> {
    let group = app_state.db.group.get_group(&user_id, &group_id).await?;
    Ok(Json(group))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GroupAndMembers {
    pub group: GroupInfo,
    pub members: Vec<GroupMember>,
}

pub async fn get_group_and_members(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, group_id)): PathWithAuthExtractor<(String, String)>,
) -> Result<Json<GroupAndMembers>, Error> {
    let group = app_state
        .db
        .group
        .get_group_and_members(&user_id, &group_id)
        .await?;
    let info = group.group.ok_or(Error::not_found())?;
    Ok(Json(GroupAndMembers {
        group: info,
        members: group.members,
    }))
}

pub async fn get_group_members(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<GetMemberReq>,
) -> Result<Json<Vec<GroupMember>>, Error> {
    let members = app_state
        .db
        .group
        .get_members(&req.user_id, &req.group_id, req.mem_ids)
        .await?;

    Ok(Json(members))
}

///  use the send_message, need to store the notification message
/// create a new record handler
pub async fn create_group_handler(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(user_id): PathWithAuthExtractor<String>,
    JsonWithAuthExtractor(mut new_group): JsonWithAuthExtractor<GroupCreate>,
) -> Result<Json<GroupInvitation>, Error> {
    // filter the empty item
    new_group.members_id.retain(|v| !v.is_empty());

    let group_id = nanoid!();
    new_group.id.clone_from(&group_id);
    // put the owner to the group members
    new_group.members_id.push(user_id.clone());

    let group_id = new_group.id.clone();
    // insert group information and members information to db
    let invitation = app_state
        .db
        .group
        .create_group_with_members(&new_group)
        .await?;
    let members_id = invitation.members.iter().fold(
        Vec::with_capacity(invitation.members.len()),
        |mut acc, member| {
            acc.push(member.user_id.clone());
            acc
        },
    );
    // todo save the information to message receive box

    //todo save the information to cache, is it necessary to save the group info to cache?

    // save members id
    app_state
        .cache
        .save_group_members_id(&group_id, members_id)
        .await?;

    let mut chat_rpc = app_state.chat_rpc.clone();
    let msg = bincode::serialize(&invitation)?;

    // increase the send sequence for sender
    let (seq, _, _) = app_state.cache.incr_send_seq(&user_id).await?;

    // send the group invitation to the members
    let request = SendMsgRequest::new_with_group_invitation(user_id, group_id, seq, msg);

    chat_rpc.send_msg(request).await?;

    Ok(Json(invitation))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GroupInviteNewResponse {
    pub group_id: String,
    pub members: Vec<GroupMember>,
}

pub async fn invite_new_members(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(invitation): JsonWithAuthExtractor<GroupInviteNew>,
) -> Result<(), Error> {
    let user_id = invitation.user_id.clone();
    let group_id = invitation.group_id.clone();
    let members = invitation.members.clone();
    // update members
    app_state.db.group.invite_new_members(&invitation).await?;

    // update cache
    // WE SHOULD ADD NEW MEMBERS TO CACHE
    // SO THAT THE CONSUMER CAN GET THE MEMBERS' ID
    app_state
        .cache
        .save_group_members_id(&invitation.group_id, invitation.members)
        .await?;

    let mut chat_rpc = app_state.chat_rpc.clone();

    let msg = bincode::serialize(&members)?;

    // increase the send sequence for sender
    let (seq, _, _) = app_state.cache.incr_send_seq(&user_id).await?;

    let request = SendMsgRequest::new_with_group_invite_new(user_id, group_id, seq, msg);
    chat_rpc.send_msg(request).await?;

    Ok(())
}

pub async fn update_group_handler(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(user_id): PathWithAuthExtractor<String>,
    JsonWithAuthExtractor(group_update): JsonWithAuthExtractor<GroupUpdate>,
) -> Result<Json<GroupInfo>, Error> {
    // update db
    let group_info = app_state.db.group.update_group(&group_update).await?;

    //todo notify the group members, except updater
    // let mut members = app_state.cache.query_group_members_id(&inner.id).await?;
    let mut chat_rpc = app_state.chat_rpc.clone();
    // notify members, except self
    let msg = bincode::serialize(&group_info)?;

    // increase the send sequence for sender
    let (seq, _, _) = app_state.cache.incr_send_seq(&user_id).await?;

    let req = SendMsgRequest::new_with_group_update(user_id, group_info.id.clone(), seq, msg);
    chat_rpc.send_msg(req).await?;
    Ok(Json(group_info))
}

pub async fn remove_member(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<RemoveMemberRequest>,
) -> Result<(), Error> {
    let req_cloned = req.clone();

    app_state
        .db
        .group
        .remove_member(&req.group_id, &req.user_id, &req.mem_id)
        .await?;

    // send remove message to group members
    let mut chat_rpc = app_state.chat_rpc.clone();
    let msg = bincode::serialize(&req_cloned.mem_id)?;

    // increase the send sequence for sender
    let (seq, _, _) = app_state.cache.incr_send_seq(&req_cloned.user_id).await?;

    let request = SendMsgRequest::new_with_group_remove_mem(
        req_cloned.user_id,
        req_cloned.group_id,
        seq,
        msg,
    );
    chat_rpc.send_msg(request).await?;

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DeleteGroupRequest {
    pub user_id: String,
    pub group_id: String,
    pub is_dismiss: bool,
}

pub async fn delete_group_handler(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(group): JsonWithAuthExtractor<DeleteGroupRequest>,
) -> Result<(), Error> {
    let msg = if group.is_dismiss {
        app_state
            .db
            .group
            .delete_group(&group.group_id, &group.user_id)
            .await?;

        MsgType::GroupDismiss
    } else {
        // exit group
        app_state
            .db
            .group
            .exit_group(&group.user_id, &group.group_id)
            .await?;
        MsgType::GroupMemberExit
    };

    let mut chat_rpc = app_state.chat_rpc.clone();
    // notify members, except self

    // increase the send sequence for sender
    let (seq, _, _) = app_state.cache.incr_send_seq(&group.user_id).await?;
    let ws_req = SendMsgRequest::new_with_group_operation(group.user_id, group.group_id, msg, seq);
    chat_rpc.send_msg(ws_req).await?;

    Ok(())
}

// 新增 - 群组设置相关
pub async fn update_group_settings(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<UpdateGroupRequest>,
) -> Result<Json<GroupInfo>, Error> {
    let user_id = req.user_id.clone();
    let group_id = req.group_id.clone();

    // 更新设置
    let group_info = app_state.db.group.update_group_settings(&req).await?;

    // 通知群组成员
    let mut chat_rpc = app_state.chat_rpc.clone();
    let msg = bincode::serialize(&group_info)?;

    // 增加发送序列
    let (seq, _, _) = app_state.cache.incr_send_seq(&user_id).await?;

    let req = SendMsgRequest::new_with_group_update(user_id, group_id, seq, msg);
    chat_rpc.send_msg(req).await?;

    Ok(Json(group_info))
}

pub async fn update_member_settings(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<UpdateMemberSettingsRequest>,
) -> Result<Json<GroupMember>, Error> {
    let member = app_state
        .db
        .group
        .update_member_settings(
            &req.group_id,
            &req.user_id,
            &req.notification_level,
            req.display_order,
        )
        .await?;

    Ok(Json(member))
}

// 新增 - 群组分类相关
pub async fn create_group_category(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<CreateGroupCategoryRequest>,
) -> Result<Json<GroupCategory>, Error> {
    let category = app_state.db.group.create_group_category(&req).await?;
    Ok(Json(category))
}

pub async fn update_group_category(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(category): JsonWithAuthExtractor<GroupCategory>,
) -> Result<Json<GroupCategory>, Error> {
    let updated = app_state.db.group.update_group_category(&category).await?;
    Ok(Json(updated))
}

pub async fn delete_group_category(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(id): PathWithAuthExtractor<String>,
) -> Result<(), Error> {
    app_state.db.group.delete_group_category(&id).await?;
    Ok(())
}

pub async fn get_group_categories(
    State(app_state): State<AppState>,
) -> Result<Json<Vec<GroupCategory>>, Error> {
    let categories = app_state.db.group.get_group_categories().await?;
    Ok(Json(categories))
}

// 新增 - 群组文件相关
pub async fn upload_group_file(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<GroupFileUploadRequest>,
) -> Result<Json<GroupFile>, Error> {
    // 处理文件上传
    let file = app_state.db.group.upload_group_file(&req).await?;

    // 通知群组成员
    let mut chat_rpc = app_state.chat_rpc.clone();
    let msg = bincode::serialize(&file)?;

    // 增加发送序列
    let (seq, _, _) = app_state.cache.incr_send_seq(&req.uploader_id).await?;

    let notification_req = SendMsgRequest::new_with_group_file(
        req.uploader_id.clone(),
        req.group_id.clone(),
        seq,
        msg,
    );
    chat_rpc.send_msg(notification_req).await?;

    Ok(Json(file))
}

pub async fn delete_group_file(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<DeleteFileRequest>,
) -> Result<(), Error> {
    app_state
        .db
        .group
        .delete_group_file(&req.file_id, &req.user_id)
        .await?;
    Ok(())
}

pub async fn get_group_files(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(group_id): PathWithAuthExtractor<String>,
) -> Result<Json<Vec<GroupFile>>, Error> {
    let files = app_state.db.group.get_group_files(&group_id).await?;
    Ok(Json(files))
}

pub async fn update_file_pin_status(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<UpdateFilePinRequest>,
) -> Result<Json<GroupFile>, Error> {
    let file = app_state
        .db
        .group
        .update_file_pin_status(&req.file_id, req.is_pinned)
        .await?;
    Ok(Json(file))
}

// 新增 - 群组投票相关
pub async fn create_poll(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<CreatePollRequest>,
) -> Result<Json<GroupPoll>, Error> {
    let poll = app_state.db.group.create_poll(&req).await?;

    // 通知群组成员
    let mut chat_rpc = app_state.chat_rpc.clone();
    let msg = bincode::serialize(&poll)?;

    let user_id = req.creator_id.clone();
    let group_id = req.group_id.clone();

    // 增加发送序列
    let (seq, _, _) = app_state.cache.incr_send_seq(&user_id).await?;

    let notification_req = SendMsgRequest::new_with_group_poll(user_id, group_id, seq, msg);
    chat_rpc.send_msg(notification_req).await?;

    Ok(Json(poll))
}

pub async fn vote_poll(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<VotePollRequest>,
) -> Result<Json<GroupPoll>, Error> {
    let poll = app_state.db.group.vote_poll(&req).await?;
    Ok(Json(poll))
}

pub async fn close_poll(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<ClosePollRequest>,
) -> Result<Json<GroupPoll>, Error> {
    let poll = app_state
        .db
        .group
        .close_poll(&req.poll_id, &req.user_id)
        .await?;
    Ok(Json(poll))
}

pub async fn get_group_polls(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(group_id): PathWithAuthExtractor<String>,
) -> Result<Json<Vec<GroupPoll>>, Error> {
    let polls = app_state.db.group.get_group_polls(&group_id).await?;
    Ok(Json(polls))
}

pub async fn get_poll_details(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(poll_id): PathWithAuthExtractor<String>,
) -> Result<Json<GroupPoll>, Error> {
    let poll = app_state.db.group.get_poll_details(&poll_id).await?;
    Ok(Json(poll))
}

// 新增 - 群组禁言相关
pub async fn mute_group_member(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<MuteGroupMemberRequest>,
) -> Result<Json<GroupMuteRecord>, Error> {
    let record = app_state.db.group.mute_group_member(&req).await?;

    // 通知群组成员
    let mut chat_rpc = app_state.chat_rpc.clone();
    let msg = bincode::serialize(&record)?;

    // 增加发送序列
    let (seq, _, _) = app_state.cache.incr_send_seq(&req.operator_id).await?;

    let notification_req = SendMsgRequest::new_with_group_mute(
        req.operator_id.clone(),
        req.group_id.clone(),
        seq,
        msg,
    );
    chat_rpc.send_msg(notification_req).await?;

    Ok(Json(record))
}

pub async fn unmute_group_member(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<UnmuteRequest>,
) -> Result<(), Error> {
    app_state
        .db
        .group
        .unmute_group_member(&req.group_id, &req.user_id, &req.operator_id)
        .await?;
    Ok(())
}

pub async fn get_group_muted_members(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(group_id): PathWithAuthExtractor<String>,
) -> Result<Json<Vec<GroupMuteRecord>>, Error> {
    let records = app_state
        .db
        .group
        .get_group_muted_members(&group_id)
        .await?;
    Ok(Json(records))
}

// 新增 - 群组公告相关
pub async fn create_announcement(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<CreateAnnouncementRequest>,
) -> Result<Json<GroupAnnouncement>, Error> {
    let announcement = app_state.db.group.create_announcement(&req).await?;

    // 通知群组成员
    let mut chat_rpc = app_state.chat_rpc.clone();
    let msg = bincode::serialize(&announcement)?;

    // 增加发送序列
    let (seq, _, _) = app_state.cache.incr_send_seq(&req.creator_id).await?;

    let notification_req = SendMsgRequest::new_with_group_announcement(
        req.creator_id.clone(),
        req.group_id.clone(),
        seq,
        msg,
    );
    chat_rpc.send_msg(notification_req).await?;

    Ok(Json(announcement))
}

pub async fn delete_announcement(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<DeleteAnnouncementRequest>,
) -> Result<(), Error> {
    app_state
        .db
        .group
        .delete_announcement(&req.id, &req.user_id)
        .await?;
    Ok(())
}

pub async fn get_group_announcements(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(group_id): PathWithAuthExtractor<String>,
) -> Result<Json<Vec<GroupAnnouncement>>, Error> {
    let announcements = app_state
        .db
        .group
        .get_group_announcements(&group_id)
        .await?;
    Ok(Json(announcements))
}

pub async fn pin_announcement(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<PinAnnouncementRequest>,
) -> Result<Json<GroupAnnouncement>, Error> {
    let announcement = app_state
        .db
        .group
        .pin_announcement(&req.id, req.is_pinned)
        .await?;
    Ok(Json(announcement))
}

// 新增 - 成员管理增强
pub async fn change_member_role(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<ChangeMemberRoleRequest>,
) -> Result<Json<GroupMember>, Error> {
    let member = app_state
        .db
        .group
        .change_member_role(&req.group_id, &req.user_id, &req.target_id, &req.new_role)
        .await?;
    Ok(Json(member))
}

// 辅助请求结构体
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateMemberSettingsRequest {
    pub group_id: String,
    pub user_id: String,
    pub notification_level: String,
    pub display_order: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteFileRequest {
    pub file_id: String,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateFilePinRequest {
    pub file_id: String,
    pub is_pinned: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClosePollRequest {
    pub poll_id: String,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnmuteRequest {
    pub group_id: String,
    pub user_id: String,
    pub operator_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteAnnouncementRequest {
    pub id: String,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PinAnnouncementRequest {
    pub id: String,
    pub is_pinned: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChangeMemberRoleRequest {
    pub group_id: String,
    pub user_id: String,   // 操作者
    pub target_id: String, // 被操作者
    pub new_role: String,
}
