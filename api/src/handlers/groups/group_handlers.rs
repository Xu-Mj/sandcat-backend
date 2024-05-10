use axum::extract::State;
use axum::Json;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

use abi::errors::Error;
use abi::message::{
    GroupCreate, GroupCreateRequest, GroupDeleteRequest, GroupInfo, GroupInvitation,
    GroupInviteNew, GroupInviteNewRequest, GroupMember, GroupUpdate, GroupUpdateRequest, MsgType,
    SendMsgRequest, UserAndGroupId,
};

use crate::api_utils::custom_extract::{JsonWithAuthExtractor, PathWithAuthExtractor};
use crate::AppState;

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
    let response = db_rpc.group_create(request).await.map_err(|e| {
        Error::InternalServer(format!(
            "procedure db rpc service error: group_create {:?}",
            e
        ))
    })?;
    let invitation = response
        .into_inner()
        .invitation
        .ok_or_else(|| Error::InternalServer("group create failed".to_string()))?;

    let mut chat_rpc = app_state.chat_rpc.clone();
    let msg = bincode::serialize(&invitation).map_err(|e| Error::InternalServer(e.to_string()))?;

    // send the group invitation to the members
    let request = SendMsgRequest::new_with_group_invitation(user_id, group_id, msg);

    chat_rpc.send_msg(request).await.map_err(|e| {
        Error::InternalServer(format!(
            "procedure chat rpc service error: send_msg {:?}",
            e
        ))
    })?;

    Ok(Json(invitation))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GroupInviteNewResponse {
    pub user_id: String,
    pub group_id: String,
    pub members: Vec<GroupMember>,
}

pub async fn invite_new_members(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(invitation): JsonWithAuthExtractor<GroupInviteNew>,
) -> Result<(), Error> {
    let user_id = invitation.user_id.clone();
    let group_id = invitation.group_id.clone();
    // update members
    let mut db_rpc = app_state.db_rpc.clone();
    let request = GroupInviteNewRequest {
        group_invite: Some(invitation),
    };
    let response = db_rpc.group_invite_new(request).await.map_err(|e| {
        Error::InternalServer(format!(
            "procedure db rpc service error: group_invite_new {:?}",
            e
        ))
    })?;

    let members = response.into_inner().members;
    let new_invite = GroupInviteNewResponse {
        user_id,
        group_id,
        members,
    };
    let mut chat_rpc = app_state.chat_rpc.clone();

    let msg = bincode::serialize(&new_invite).map_err(|e| Error::InternalServer(e.to_string()))?;

    let request =
        SendMsgRequest::new_with_group_invite_new(new_invite.user_id, new_invite.group_id, msg);
    chat_rpc.send_msg(request).await.map_err(|e| {
        Error::InternalServer(format!(
            "procedure chat rpc service error: send_msg {:?}",
            e
        ))
    })?;

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
    let response = db_rpc.group_update(request).await.map_err(|e| {
        Error::InternalServer(format!(
            "procedure db rpc service error: group_update {:?}",
            e
        ))
    })?;

    let inner = response.into_inner().group.ok_or_else(|| {
        Error::InternalServer("group update failed, rpc response is none".to_string())
    })?;

    //todo notify the group members, except updater
    // let mut members = app_state.cache.query_group_members_id(&inner.id).await?;
    let mut chat_rpc = app_state.chat_rpc.clone();
    // notify members, except self
    let msg = bincode::serialize(&inner).map_err(|e| Error::InternalServer(e.to_string()))?;
    let req = SendMsgRequest::new_with_group_update(user_id, inner.id.clone(), msg);
    chat_rpc.send_msg(req).await.map_err(|e| {
        Error::InternalServer(format!(
            "procedure chat rpc service error: send_msg {:?}",
            e
        ))
    })?;
    Ok(Json(inner))
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
        db_rpc.group_delete(req).await.map_err(|e| {
            Error::InternalServer(format!(
                "procedure db rpc service error: group_delete {:?}",
                e
            ))
        })?;
        MsgType::GroupDismiss
    } else {
        // exit group
        let req = UserAndGroupId::new(group.user_id.clone(), group.group_id.clone());
        //todo if the group already dismissed, return success directly
        db_rpc.group_member_exit(req.clone()).await.map_err(|e| {
            Error::InternalServer(format!(
                "procedure db rpc service error: group_member_exit {:?}",
                e
            ))
        })?;
        MsgType::GroupMemberExit
    };

    let mut chat_rpc = app_state.chat_rpc.clone();
    // notify members, except self
    let ws_req = SendMsgRequest::new_with_group_operation(group.user_id, group.group_id, msg);
    chat_rpc.send_msg(ws_req).await.map_err(|e| {
        Error::InternalServer(format!(
            "procedure chat rpc service error: send_msg {:?}",
            e
        ))
    })?;

    Ok(())
}
