use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::error;
use tracing::log::debug;

use crate::domain::model::friends::FriendError;
use crate::domain::model::msg::{GroupInvitation, GroupMsg, Msg};
use crate::infra::errors::InfraError;
use crate::infra::repositories::group_members::{exit_group, query_group_members_id};
use crate::infra::repositories::groups::{
    create_group_with_members, delete_group, update_group, GroupDb,
};
use crate::utils::redis::redis_crud::{del_group, store_group};
use crate::utils::{JsonWithAuthExtractor, PathWithAuthExtractor};
use crate::AppState;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GroupRequest {
    pub id: String,
    pub owner: String,
    pub avatar: String,
    pub group_name: String,
    pub members_id: Vec<String>,
}

/// implement From<GroupDb> for GroupRequest
impl From<GroupDb> for GroupRequest {
    fn from(value: GroupDb) -> Self {
        GroupRequest {
            id: value.id,
            owner: value.owner,
            avatar: value.avatar,
            group_name: value.name,
            members_id: vec![], // value.members.split(',').map(String::from).collect(),
        }
    }
}

/// create a new record handler
pub async fn create_group_handler(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(user_id): PathWithAuthExtractor<String>,
    JsonWithAuthExtractor(mut new_group): JsonWithAuthExtractor<GroupRequest>,
) -> Result<Json<GroupInvitation>, FriendError> {
    let mut members_id = std::mem::take(&mut new_group.members_id);
    let cloned_ids = members_id.clone();
    members_id.push(user_id);
    let group = GroupInvitation::from(new_group);
    let group_db = create_group_with_members(&app_state.pg_pool, group, members_id)
        .await
        .map_err(|err| FriendError::InternalServerError(err.to_string()))?;
    // send message by async way
    let msg = group_db.clone();
    let hub = app_state.hub.clone();
    let redis = app_state.redis.clone();
    tokio::spawn(async move {
        if let Ok(conn) = redis.get_connection() {
            // store data to redis
            store_group(conn, &msg)
                .map_err(|err| error!("store_user_views error: {:#?}", err))
                .unwrap();
            debug!("group creation success");
            // send it to online users
            hub.send_group(&cloned_ids, &Msg::Group(GroupMsg::Invitation(msg)))
                .await;
        }
    });

    Ok(Json(group_db))
}

pub async fn update_group_handler(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(user_id): PathWithAuthExtractor<String>,
    JsonWithAuthExtractor(group_info): JsonWithAuthExtractor<GroupRequest>,
) -> Result<Json<GroupDb>, FriendError> {
    let group_db = GroupDb::from(group_info);
    println!("user id: {user_id}");
    match update_group(&app_state.pg_pool, &group_db).await {
        Ok(group) => Ok(Json(group)),
        Err(e) => match e {
            InfraError::NotFound => Err(FriendError::NotFound(group_db.id)),
            _ => Err(FriendError::InternalServerError(e.to_string())),
        },
    }
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
) -> Result<Json<()>, FriendError> {
    let mut members = Vec::new();

    // query members id
    if let Ok(mut conn) = app_state.redis.get_connection() {
        if let Ok(mut list) =
            query_group_members_id(&mut conn, &app_state.pg_pool, group.group_id.clone()).await
        {
            list.retain(|v| v != &group.user_id);
            tracing::debug!("members: {:#?}", list);
            members = list;
        }
    }

    let msg = if group.is_dismiss {
        let group_id = group.group_id.clone();

        // delete group
        match delete_group(&app_state.pg_pool, &group_id, &group.user_id).await {
            Ok(_) => {
                // delete group information and members from redis
                if let Ok(conn) = app_state.redis.get_connection() {
                    if let Err(e) = del_group(conn, &group_id) {
                        error!("delete group error: {:#?}", e);
                    }
                }
                GroupMsg::Dismiss(group_id)
            }
            Err(e) => {
                return match e {
                    InfraError::NotFound => Err(FriendError::NotFound(group_id)),
                    _ => Err(FriendError::InternalServerError(e.to_string())),
                }
            }
        }
    } else {
        // exit group
        if let Err(e) = exit_group(&app_state.pg_pool, &group.group_id, &group.user_id).await {
            return Err(FriendError::InternalServerError(e.to_string()));
        }
        GroupMsg::MemberExit((group.user_id, group.group_id))
    };
    // notify members
    debug!("delete group success; send group message");
    app_state.hub.send_group(&members, &Msg::Group(msg)).await;
    Ok(Json(()))
}
