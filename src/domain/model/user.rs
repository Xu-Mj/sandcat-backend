use crate::infra::errors::InfraError;
use crate::infra::repositories::user_repo::UserDb;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Serialize, Default, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub account: String,
    pub avatar: String,
    pub gender: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub birthday: Option<chrono::NaiveDateTime>,
    pub create_time: chrono::NaiveDateTime,
    pub update_time: chrono::NaiveDateTime,
    // pub is_delete: bool,
}

impl From<UserDb> for User {
    fn from(user_db: UserDb) -> Self {
        Self {
            id: user_db.id,
            name: user_db.name,
            avatar: user_db.avatar,
            account: user_db.account,
            gender: user_db.gender,
            phone: user_db.phone,
            email: user_db.email,
            address: user_db.address,
            birthday: user_db.birthday,
            create_time: user_db.create_time,
            update_time: user_db.update_time,
        }
    }
}
type ID = i32;
pub enum UserError {
    InternalServerError(String),
    NotFound(ID),
    InfraError(InfraError),
}

// 将用户错误转为axum响应
impl IntoResponse for UserError {
    fn into_response(self) -> Response {
        let (status, err_msg) = match self {
            UserError::InternalServerError(err_msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal Server Error: {}", err_msg),
            ),
            UserError::NotFound(id) => (StatusCode::NOT_FOUND, format!(" User {} Not Found", id)),
            UserError::InfraError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal Server Error: {}", err),
            ),
        };
        (
            status,
            Json(json!({"resource":"UserModel","message":err_msg})),
        )
            .into_response()
    }
}
