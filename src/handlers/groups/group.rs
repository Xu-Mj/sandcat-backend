use crate::domain::model::friends::FriendError;
use crate::infra::repositories::groups::{create_group, GroupDb};
use crate::utils::JsonWithAuthExtractor;
use crate::AppState;
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

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
    let group = GroupDb::from(new_group);
    let group_db = create_group(&pool, group)
        .await
        .map_err(|err| FriendError::InternalServerError(err.to_string()))?;
    Ok(Json(group_db.into()))
}
