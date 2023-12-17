use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::error::Error;

type Msg = String;
type Location = String;
type Path = String;
/// 全局变量
#[derive(Debug)]
pub enum AppError {
    // 500
    InternalServer(String),
    // 400
    BodyParsing(Msg, Path),
    PathParsing(Msg, Option<Location>),
}

pub fn internal_error<E: Error>(err: E) -> AppError {
    AppError::InternalServer(err.to_string())
}
// 实现axum的into response特征，将自定义错误转为axum的响应
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::InternalServer(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("INTERNAL SERVER ERROR: {}", msg),
            ),
            AppError::BodyParsing(msg, path) => (
                StatusCode::BAD_REQUEST,
                format!("Bad Request error: {{ message: {}, path: {}}}", msg, path),
            ),
            AppError::PathParsing(msg, location) => {
                let mut msg = format!("Bad Request error: {}", msg);
                if location.is_some() {
                    msg = format!("Bad Request error: in {} : {}", location.unwrap(), msg);
                }
                (StatusCode::BAD_REQUEST, msg)
            }
        };
        (status, Json(json!({"message":msg}))).into_response()
    }
}
