use abi::errors::Error;
use axum::{
    async_trait,
    extract::{path::ErrorKind, rejection::PathRejection, FromRequestParts},
    http::{request::Parts, StatusCode},
};
use serde::de::DeserializeOwned;

// We define our own `Path` extractor that customizes the errors from `axum::extract::Path`
pub struct PathExtractor<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for PathExtractor<T>
where
    // these trait bounds are copied from `impl FromRequest for axum::extract::path::Path`
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
                        let mut status = StatusCode::BAD_REQUEST;

                        let kind = inner.into_kind();
                        let body = match &kind {
                            ErrorKind::WrongNumberOfParameters { .. } => {
                                Error::PathParsing(kind.to_string(), None)
                            }

                            ErrorKind::ParseErrorAtKey { key, .. } => {
                                Error::PathParsing(kind.to_string(), Some(key.clone()))
                            }

                            ErrorKind::ParseErrorAtIndex { index, .. } => {
                                Error::PathParsing(kind.to_string(), Some(index.to_string()))
                            }

                            ErrorKind::ParseError { .. } => {
                                Error::PathParsing(kind.to_string(), None)
                            }

                            ErrorKind::InvalidUtf8InPathParam { key } => {
                                Error::PathParsing(kind.to_string(), Some(key.clone()))
                            }

                            ErrorKind::UnsupportedType { .. } => {
                                // this errors is caused by the programmer using an unsupported type
                                // (such as nested maps) so respond with `500` instead
                                status = StatusCode::INTERNAL_SERVER_ERROR;
                                Error::InternalServer(kind.to_string())
                            }

                            ErrorKind::Message(msg) => Error::PathParsing(msg.clone(), None),
                            _ => Error::PathParsing(
                                format!("Unhandled deserialization errors: {kind}"),
                                None,
                            ),
                        };

                        (status, body)
                    }
                    PathRejection::MissingPathParams(error) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Error::PathParsing(error.to_string(), None),
                    ),
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Error::PathParsing(format!("Unhandled path rejection: {rejection}"), None),
                    ),
                };

                Err((status, body))
            }
        }
    }
}
