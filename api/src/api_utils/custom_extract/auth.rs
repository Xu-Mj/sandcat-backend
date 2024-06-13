use axum::extract::path::ErrorKind;
use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{FromRef, FromRequestParts, Request};
use axum::http::request::Parts;
use axum::{
    async_trait,
    extract::{FromRequest, MatchedPath},
    http::StatusCode,
    RequestPartsExt,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::de::DeserializeOwned;

use abi::errors::Error;

use crate::handlers::users::Claims;
use crate::AppState;

pub struct JsonWithAuthExtractor<T>(pub T);

const AUTHORIZATION_HEADER: &str = "Authorization";
const BEARER: &str = "Bearer";

// The AUTHENTICATION here uses the browser fingerprint plus the user ID,
// otherwise it is not reasonable to force a user to log off if the computer
// is permanently on before the expiration date
// When the user closes the page/app, record the user's closing time.
// The next time the app opens, determine the time interval.
// If it is more than seven days, you need to log in again

#[async_trait]
impl<S, T> FromRequest<S> for JsonWithAuthExtractor<T>
where
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Error);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (mut parts, body) = req.into_parts();
        let path = parts
            .extract::<MatchedPath>()
            .await
            .map(|path| path.as_str().to_owned())
            .ok()
            .unwrap_or(String::new());
        let app_state = AppState::from_ref(state);

        if let Some(header) = parts.headers.get(AUTHORIZATION_HEADER) {
            // analyze the header
            let header = header.to_str().unwrap_or("");
            if !header.starts_with(BEARER) {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Error::UnAuthorized("UnAuthorized Request".to_string(), path),
                ));
            }
            let header: Vec<&str> = header.split_whitespace().collect();

            if let Err(err) = decode::<Claims>(
                header[1],
                &DecodingKey::from_secret(app_state.jwt_secret.as_bytes()),
                &Validation::default(),
            ) {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Error::InternalServer(err.to_string()),
                ));
            }

            let req = Request::from_parts(parts, body);

            match axum::Json::<T>::from_request(req, state).await {
                Ok(value) => Ok(Self(value.0)),
                // convert the errors from `axum::Json` into whatever we want
                Err(rejection) => {
                    let app_err = Error::BodyParsing(rejection.body_text(), path);
                    Err((rejection.status(), app_err))
                }
            }
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                Error::UnAuthorized("UnAuthorized Request".to_string(), path),
            ))
        }
    }
}

pub struct PathWithAuthExtractor<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for PathWithAuthExtractor<T>
where
    // these trait bounds are copied from `rpc FromRequest for axum::extract::path::Path`
    T: DeserializeOwned + Send,
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Error);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let path = parts
            .extract::<MatchedPath>()
            .await
            .map(|path| path.as_str().to_owned())
            .ok()
            .unwrap_or(String::new());
        let app_state = AppState::from_ref(state);

        if let Some(header) = parts.headers.get(AUTHORIZATION_HEADER) {
            // 解析请求头
            let header = header.to_str().unwrap_or("");
            if !header.starts_with(BEARER) {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Error::UnAuthorized("UnAuthorized Request".to_string(), path),
                ));
            }

            let header: Vec<&str> = header.split_whitespace().collect();

            if let Err(err) = decode::<Claims>(
                header[1],
                &DecodingKey::from_secret(app_state.jwt_secret.as_bytes()),
                &Validation::default(),
            ) {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Error::InternalServer(err.to_string()),
                ));
            }

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
                                    // these errors are caused by the programmer using an unsupported type
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
                            Error::PathParsing(
                                format!("Unhandled path rejection: {rejection}"),
                                None,
                            ),
                        ),
                    };

                    Err((status, body))
                }
            }
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                Error::UnAuthorized("UnAuthorized Request".to_string(), path),
            ))
        }
    }
}
