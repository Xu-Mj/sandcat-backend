use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::domain::model::friends::FriendError;
use crate::domain::model::msg::{CreateGroup, Msg};
use crate::infra::repositories::groups::{create_group_with_members, GroupDb};
use crate::utils::JsonWithAuthExtractor;
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
    JsonWithAuthExtractor(mut new_group): JsonWithAuthExtractor<GroupRequest>,
) -> Result<Json<CreateGroup>, FriendError> {
    let members_id = std::mem::take(&mut new_group.members_id);
    let group = CreateGroup::from(new_group);
    let group_db = create_group_with_members(&app_state.pg_pool, group, members_id.clone())
        .await
        .map_err(|err| FriendError::InternalServerError(err.to_string()))?;
    // send message by async way
    let msg = group_db.clone();
    let hub = app_state.hub.clone();
    tokio::spawn(async move {
        // if let Ok(redis) = redis.get_connection() {
        // select group members info

        // store data to redis
        // store_group_mems(redis, &result)
        //     .map_err(|err| error!("store_user_views error: {:#?}", err))
        //     .unwrap();
        // send it to online users
        hub.send_group(&members_id, &Msg::CreateGroup(msg)).await;
        // }
    });

    // notify other users
    // make a message
    Ok(Json(group_db))
}
