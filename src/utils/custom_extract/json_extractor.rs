use axum::extract::rejection::JsonRejection;
// 定义一个提取器，主要作用是用来处理错误，将axum抛出的错误转为我们自定义的错误
use crate::errors::AppError;
use axum_macros::FromRequest;

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct JsonExtractor<T>(pub T);

// 实现From特征以实现从J送Rejection到自定义错误的转换
impl From<JsonRejection> for AppError {
    fn from(value: JsonRejection) -> Self {
        AppError::BodyParsingError(value.to_string())
    }
}
