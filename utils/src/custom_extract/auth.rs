use abi::errors::Error;
use axum::extract::path::ErrorKind;
use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{FromRequestParts, Request};
use axum::http::request::Parts;
use axum::{
    async_trait,
    extract::{FromRequest, MatchedPath},
    http::StatusCode,
    RequestPartsExt,
};
use serde::de::DeserializeOwned;

pub struct JsonWithAuthExtractor<T>(pub T);

const AUTHORIZATION_HEADER: &str = "Authorization";
const BEARER: &str = "Bearer";

// 这里的鉴权采用浏览器指纹加过用户标识，否则如果一个用户永久开着电脑到达过期时间的话直接强制下线是不合理的
// 在用户关闭网页、应用时记录用户关闭时间，下次打开时判断时间间隔，超过七天则需要重新登录
#[async_trait]
impl<S, T> FromRequest<S> for JsonWithAuthExtractor<T>
where
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
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
            tracing::debug!("header : {}", header[1]);

            // let claim = match decode::<Claims>(
            //     header[1],
            //     &DecodingKey::from_secret(CONFIG.get().unwrap().jwt_secret().as_bytes()),
            //     &Validation::default(),
            // ) {
            //     Ok(data) => data,
            //     Err(err) => match err.kind() {
            //         ErrorKind::ExpiredSignature => {
            //         }
            //         _ => {
            //             return Err((
            //                 StatusCode::INTERNAL_SERVER_ERROR,
            //                 Error::InternalServer(err.to_string()),
            //             ));
            //         }
            //     },
            // };
            // // 判断签名是否过期
            // if chrono::Local::now().timestamp_millis() as u64 - claim.claims.iat
            //     > claim.claims.update
            // {
            //     return Err((
            //         StatusCode::UNAUTHORIZED,
            //         Error::UnAuthorized("UnAuthorized Request".to_string(), path),
            //     ));
            // }
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
            tracing::debug!("header : {}", header[1]);
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
