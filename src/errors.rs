use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::error::Error;

/// 全局变量
#[derive(Debug)]
pub enum AppError {
    // 500
    InternalServerError(String),
    // 400
    BodyParsingError(String),
}

pub fn internal_error<E: Error>(err: E) -> AppError {
    AppError::InternalServerError(err.to_string())
}
// 实现axum的into response特征，将自定义错误转为axum的响应
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::InternalServerError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("INTERNAL SERVER ERROR: {}", msg),
            ),
            AppError::BodyParsingError(msg) => (
                StatusCode::BAD_REQUEST,
                format!("Bad Request error: {}", msg),
            ),
        };
        (status, Json(json!({"message":msg}))).into_response()
    }
}
