use axum::extract::State;
use axum::Json;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

use abi::errors::Error;
use abi::message::{
    GetGroupRequest, GetMemberReq, GroupCreate, GroupCreateRequest, GroupDeleteRequest, GroupInfo,
    GroupInvitation, GroupInviteNew, GroupInviteNewRequest, GroupMember, GroupUpdate,
    GroupUpdateRequest, MsgType, RemoveMemberRequest, SendMsgRequest, UserAndGroupId,
};

use crate::api_utils::custom_extract::{JsonWithAuthExtractor, PathWithAuthExtractor};
use crate::AppState;

/// get group information
/// need user id and group id
/// if user is not in the group, return error
pub async fn get_group(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, group_id)): PathWithAuthExtractor<(String, String)>,
) -> Result<Json<GroupInfo>, Error> {
    let mut db_rpc = app_state.db_rpc.clone();
    let resp = db_rpc
        .get_group(GetGroupRequest::new(user_id, group_id))
        .await?;

    let group = resp
        .into_inner()
        .group
        .ok_or(Error::internal_with_details("group not found"))?;

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
    let mut db_rpc = app_state.db_rpc.clone();

    let resp = db_rpc
        .get_group_and_members(GetGroupRequest::new(user_id, group_id))
        .await?;

    let resp = resp.into_inner();
    let group = resp
        .group
        .ok_or(Error::internal_with_details("group not found"))?;

    Ok(Json(GroupAndMembers {
        group,
        members: resp.members,
    }))
}

pub async fn get_group_members(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<GetMemberReq>,
) -> Result<Json<Vec<GroupMember>>, Error> {
    let mut db_rpc = app_state.db_rpc.clone();

    let resp = db_rpc.get_group_members(req).await?;

    Ok(Json(resp.into_inner().members))
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

    // send rpc request
    let request = GroupCreateRequest::new(new_group);
    let mut db_rpc = app_state.db_rpc.clone();
    let response = db_rpc.group_create(request).await?;

    // decode the invitation to binary
    let invitation = response
        .into_inner()
        .invitation
        .ok_or(Error::internal_with_details("group create failed"))?;

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
    let mut db_rpc = app_state.db_rpc.clone();
    let request = GroupInviteNewRequest {
        group_invite: Some(invitation),
    };
    db_rpc.group_invite_new(request).await?;

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
    JsonWithAuthExtractor(group_info): JsonWithAuthExtractor<GroupUpdate>,
) -> Result<Json<GroupInfo>, Error> {
    // send rpc request to update group
    let mut db_rpc = app_state.db_rpc.clone();
    let request = GroupUpdateRequest::new(group_info);
    let response = db_rpc.group_update(request).await?;

    let inner = response
        .into_inner()
        .group
        .ok_or(Error::internal_with_details(
            "update group error, group is none",
        ))?;

    //todo notify the group members, except updater
    // let mut members = app_state.cache.query_group_members_id(&inner.id).await?;
    let mut chat_rpc = app_state.chat_rpc.clone();
    // notify members, except self
    let msg = bincode::serialize(&inner)?;

    // increase the send sequence for sender
    let (seq, _, _) = app_state.cache.incr_send_seq(&user_id).await?;

    let req = SendMsgRequest::new_with_group_update(user_id, inner.id.clone(), seq, msg);
    chat_rpc.send_msg(req).await?;
    Ok(Json(inner))
}

pub async fn remove_member(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<RemoveMemberRequest>,
) -> Result<(), Error> {
    let req_cloned = req.clone();

    let mut db_rpc = app_state.db_rpc.clone();
    db_rpc.remove_member(req).await?;

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
    let mut db_rpc = app_state.db_rpc.clone();
    let msg = if group.is_dismiss {
        let req = GroupDeleteRequest::new(group.group_id.clone(), group.user_id.clone());
        db_rpc.group_delete(req).await?;
        MsgType::GroupDismiss
    } else {
        // exit group
        let req = UserAndGroupId::new(group.user_id.clone(), group.group_id.clone());
        //todo if the group already dismissed, return success directly
        db_rpc.group_member_exit(req.clone()).await?;
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
