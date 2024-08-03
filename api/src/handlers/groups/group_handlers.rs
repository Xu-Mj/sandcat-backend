use axum::extract::State;
use axum::Json;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

use abi::errors::Error;
use abi::message::{
    GetMemberReq, GroupCreate, GroupInfo, GroupInvitation, GroupInviteNew, GroupMember,
    GroupUpdate, MsgType, RemoveMemberRequest, SendMsgRequest,
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
