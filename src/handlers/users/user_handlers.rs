
use axum::extract::State;
use axum::Json;
use nanoid::nanoid;
use crate::domain::model::user::{User, UserError};
use crate::infra::repositories::user_repo::{get, insert, NewUserDb};
use crate::utils::{JsonExtractor, PathExtractor};
use crate::AppState;
use crate::handlers::users::UserRequest;
use crate::infra::errors::InfraError;

pub async fn create_user(
    State(app_state): State<AppState>,
    JsonExtractor(new_user): JsonExtractor<UserRequest>,
) -> Result<Json<User>, UserError> {
    // 结构体转换
    let user2db = NewUserDb {
        name: new_user.name,
        account: nanoid!(),
        avatar: new_user.avatar,
        gender: new_user.gender,
        phone: new_user.phone,
        email: new_user.email,
        address: new_user.address,
        birthday: new_user.birthday,
    };
    // repo方法调用
    let user = insert(&app_state.pool, user2db).await.map_err(UserError::InfraError)?;
    // 结果返回
    Ok(Json(user))
}

pub async fn get_user_by_id(
    State(app_state): State<AppState>,
    PathExtractor(id): PathExtractor<i32>,
) -> Result<Json<User>, UserError> {
    let user = get(&app_state.pool, id).await.map_err(|err| {
        match err {
            InfraError::InternalServerError(e) => UserError::InternalServerError(e),
            InfraError::NotFound => UserError::NotFound(id)
        }
    })?;
    Ok(Json(user))
}
