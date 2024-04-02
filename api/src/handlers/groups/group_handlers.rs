use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::error;
use tracing::log::debug;

use abi::errors::Error;
use abi::message::msg::Data;
use abi::message::{
    GroupCreate, GroupCreateRequest, GroupDeleteRequest, GroupInfo, GroupInvitation, GroupUpdate,
    GroupUpdateRequest, SendGroupMsgRequest, UserAndGroupId,
};
use utils::custom_extract::{JsonWithAuthExtractor, PathWithAuthExtractor};

use crate::AppState;

/// create a new record handler
pub async fn create_group_handler(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(user_id): PathWithAuthExtractor<String>,
    JsonWithAuthExtractor(mut new_group): JsonWithAuthExtractor<GroupCreate>,
) -> Result<Json<GroupInvitation>, Error> {
    // use it to send message to other users except owner
    let cloned_ids = new_group.members_id.clone();

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

    let mut msg_rpc = app_state.ws_rpc.clone();
    let msg = invitation.clone();
    tokio::spawn(async move {
        let request = SendGroupMsgRequest::new_with_group_invitation(user_id, msg, cloned_ids);
        // send it to online users
        if let Err(e) = msg_rpc.send_group_msg_to_user(request).await {
            error!("send_group_msg_to_user errors: {:#?}", e);
        }
    });

    Ok(Json(invitation))
}

// todo add generate update message when members update information
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
    let mut members = app_state.cache.query_group_members_id(&inner.id).await?;
    let mut ws_rpc = app_state.ws_rpc.clone();
    // notify members, except self
    let msg = inner.clone();
    tokio::spawn(async move {
        members.retain(|x| *x != user_id);
        debug!("delete group success; send group message");
        let ws_req = SendGroupMsgRequest::new_with_group_update(user_id, msg, members);
        if let Err(e) = ws_rpc.send_group_msg_to_user(ws_req).await {
            error!(
                "procedure ws rpc service error: send_group_msg_to_user {:?}",
                e
            )
        }
    });
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
    let (msg, mut members) = if group.is_dismiss {
        let req = GroupDeleteRequest::new(group.group_id.clone(), group.user_id.clone());
        let response = db_rpc.group_delete(req).await.map_err(|e| {
            Error::InternalServer(format!(
                "procedure db rpc service error: group_delete {:?}",
                e
            ))
        })?;
        let members_id = response.into_inner().members_id;
        (Data::GroupDismiss(group.group_id.clone()), members_id)
    } else {
        // exit group
        let req = UserAndGroupId::new(group.user_id.clone(), group.group_id.clone());
        let response = db_rpc.group_member_exit(req.clone()).await.map_err(|e| {
            Error::InternalServer(format!(
                "procedure db rpc service error: group_member_exit {:?}",
                e
            ))
        })?;
        let members = response.into_inner().members_id;
        (Data::GroupMemberExit(req), members)
    };

    let mut ws_rpc = app_state.ws_rpc.clone();
    // notify members, except self
    tokio::spawn(async move {
        members.retain(|x| *x != group.user_id);
        debug!("delete group success; send group message");
        let ws_req = SendGroupMsgRequest::new_with_group_msg(msg, members);
        if let Err(e) = ws_rpc.send_group_msg_to_user(ws_req).await {
            error!(
                "procedure ws rpc service error: send_group_msg_to_user {:?}",
                e
            )
        }
    });

    Ok(())
}
