use abi::errors::Error;
use axum::{
    async_trait,
    extract::{FromRequestParts, rejection::PathRejection},
    http::{StatusCode, request::Parts},
};
use serde::de::DeserializeOwned;

// We define our own `Path` extractor that customizes the errors from `axum::extract::Path`
pub struct PathExtractor<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for PathExtractor<T>
where
    // these trait bounds are copied from `rpc FromRequest for axum::extract::path::Path`
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Error);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match axum::extract::Path::<T>::from_request_parts(parts, state).await {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => {
                let (status, body) = match rejection {
                    PathRejection::FailedToDeserializePathParams(inner) => {
                        (StatusCode::BAD_REQUEST, Error::path_parsing(inner))
                    }
                    PathRejection::MissingPathParams(error) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Error::path_parsing(error),
                    ),
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Error::internal_with_details(format!(
                            "Unhandled path rejection: {rejection}"
                        )),
                    ),
                };

                Err((status, body))
            }
        }
    }
}
