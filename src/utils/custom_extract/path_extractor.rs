use axum::extract::rejection::PathRejection;
use axum_macros::FromRequestParts;

use crate::errors::AppError;

#[derive(FromRequestParts, Debug)]
#[from_request(via(axum::extract::Path), rejection(AppError))]
pub struct PathExtractor<T>(pub T);

impl From<PathRejection> for AppError {
    fn from(rejection: PathRejection) -> Self {
        AppError::BodyParsingError(rejection.to_string())
    }
}
