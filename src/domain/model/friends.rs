use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

type ID = String;

pub enum FriendError {
    InternalServerError(String),
    Parameter(String),
    NotFound(ID),
    // InfraError(InfraError),
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
            // FriendError::InfraError(err) => (
            //     StatusCode::INTERNAL_SERVER_ERROR,
            //     format!("Internal Server Error: {}", err),
            // ),
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
