use crate::infra::errors::InfraError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::TimeZone;
use serde_json::json;

// 朋友模型，对应friends表，提供对朋友的增删改查
// 好友请求，同意请求，拒绝等

pub struct Friend {
    id: i32,
    user_id: i32,
    friend_id: i32,
    status: i32,
    created_time: i64,
    updated_time: i64,
}

impl Friend {
    pub fn new(user_id: i32, friend_id: i32, status: i32) -> Friend {
        Friend {
            id: 0,
            user_id,
            friend_id,
            status,
            created_time: chrono::Local::now().timestamp_millis(),
            updated_time: chrono::Local::now().timestamp_millis(),
        }
    }
    pub fn get_id(&self) -> i32 {
        self.id
    }
}

// 提供sql接口
impl Friend {
    // pub fn get_by_id(id: i32) -> Self {
    //
    // }
}

type ID = String;

pub enum FriendError {
    InternalServerError(String),
    Parameter(String),
    NotFound(ID),
    InfraError(InfraError),
}

// 将用户错误转为axum响应
impl IntoResponse for FriendError {
    fn into_response(self) -> Response {
        let (status, err_msg) = match self {
            FriendError::InternalServerError(err_msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal Server Error: {}", err_msg),
            ),
            FriendError::NotFound(id) => (StatusCode::NOT_FOUND, format!(" User {} Not Found", id)),
            FriendError::InfraError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal Server Error: {}", err),
            ),
            FriendError::Parameter(msg) => {
                (StatusCode::BAD_REQUEST, format!("Parameter Error: {}", msg))
            }
        };
        (
            status,
            Json(json!({"resource":"FriendModel","message":err_msg})),
        )
            .into_response()
    }
}
