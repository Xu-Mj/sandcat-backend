use crate::domain::model::friends::FriendError;
use crate::domain::model::msg::{CreateGroup, Msg};
use crate::infra::repositories::groups::{create_group, GroupDb};
use crate::infra::repositories::user_repo::get_user_view_by_ids;
use crate::utils::redis::redis_crud::store_user_views;
use crate::utils::JsonWithAuthExtractor;
use crate::AppState;
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio::task::spawn_local;
use tracing::error;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GroupRequest {
    pub id: String,
    pub owner: String,
    pub avatar: Vec<String>,
    pub group_name: String,
    pub members_id: Vec<String>,
}

/// implement From<GroupDb> for GroupRequest
impl From<GroupDb> for GroupRequest {
    fn from(value: GroupDb) -> Self {
        GroupRequest {
            id: value.id,
            owner: value.owner,
            avatar: value.avatar.split(',').map(String::from).collect(),
            group_name: value.name,
            members_id: value.members.split(',').map(String::from).collect(),
        }
    }
}

/// create a new record handler
pub async fn create_group_handler(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(new_group): JsonWithAuthExtractor<GroupRequest>,
) -> Result<Json<GroupRequest>, FriendError> {
    let pool = app_state.pool.clone();
    let members_id = new_group.members_id.clone();
    let group = GroupDb::from(new_group);
    let group_db = create_group(&pool, group)
        .await
        .map_err(|err| FriendError::InternalServerError(err.to_string()))?;
    // send message by async way
    let mut msg = CreateGroup::from(group_db.clone());
    let redis = app_state
        .redis
        .get_connection()
        .map_err(|err| error!("get redis conn error: {:#?}", err))
        .unwrap();
    spawn_local(async move {
        // select group members info
        if let Ok(result) = get_user_view_by_ids(&pool, members_id.clone())
            .await
            .map_err(|err| error!("get_by_ids error: {:#?}", err))
        {
            // store data to redis
            store_user_views(redis, &result)
                .map_err(|err| error!("store_user_views error: {:#?}", err))
                .unwrap();
            msg.members = result;
            // send it to online users
            app_state
                .hub
                .send_group(&members_id, &Msg::CreateGroup(msg))
                .await;
        }
        //
    });

    let group: GroupRequest = group_db.into();
    // notify other users
    // make a message
    Ok(Json(group))
}
