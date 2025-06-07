use aws_sdk_s3::error::SdkError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
};
use mongodb::bson::document::ValueAccessError;
use serde::Serialize;
use std::error::Error as StdError;
use std::fmt;
use tonic::Status;
use tracing::error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Serialize)]
pub enum ErrorKind {
    UnknownError,
    DbError,
    ConfigReadError,
    ConfigParseError,
    NotFound,
    BroadCastError,
    InternalServer,
    BodyParsing,
    PathParsing,
    UnAuthorized,
    ParseError,
    TonicError,
    MongoDbValueAccessError,
    MongoDbBsonSerError,
    MongoDbOperateError,
    RedisError,
    IOError,
    ReqwestError,
    ServiceNotFound,
    BadRequest,
    AccountOrPassword,
    OSSError,
    CodeIsExpired,
    CodeIsInvalid,
    BinCode,
}

#[derive(Debug, Serialize)]
pub struct Error {
    kind: ErrorKind,
    details: Option<String>,
    #[serde(skip)]
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl Error {
    #[inline]
    pub fn new(
        kind: ErrorKind,
        details: impl Into<String>,
        source: impl StdError + 'static + Send + Sync,
    ) -> Self {
        Self {
            kind,
            source: Some(Box::new(source)),
            details: Some(details.into()),
        }
    }

    #[inline]
    pub fn with_kind(kind: ErrorKind) -> Self {
        Self {
            kind,
            source: None,
            details: None,
        }
    }

    #[inline]
    pub fn with_details(kind: ErrorKind, details: impl Into<String>) -> Self {
        Self {
            kind,
            source: None,
            details: Some(details.into()),
        }
    }

    #[inline]
    pub fn internal_box(error: Box<dyn StdError + 'static + Send + Sync>) -> Self {
        Self {
            kind: ErrorKind::InternalServer,
            details: Some(error.to_string()),
            source: Some(error),
        }
    }

    #[inline]
    pub fn internal(error: impl StdError + 'static + Send + Sync) -> Self {
        Self {
            kind: ErrorKind::InternalServer,
            details: Some(error.to_string()),
            source: Some(Box::new(error)),
        }
    }

    #[inline]
    pub fn broadcast(error: Box<dyn StdError + 'static + Send + Sync>) -> Self {
        Self {
            kind: ErrorKind::BroadCastError,
            details: Some(error.to_string()),
            source: Some(error),
        }
    }

    #[inline]
    pub fn internal_with_details(details: impl Into<String>) -> Self {
        Self::with_details(ErrorKind::InternalServer, details)
    }

    #[inline]
    pub fn service_not_found(name: impl Into<String>) -> Self {
        Self::with_details(ErrorKind::ServiceNotFound, name)
    }

    #[inline]
    pub fn unauthorized(
        error: impl StdError + 'static + Send + Sync,
        details: impl Into<String>,
    ) -> Self {
        Self::new(ErrorKind::UnAuthorized, details, error)
    }

    #[inline]
    pub fn unauthorized_with_details(details: impl Into<String>) -> Self {
        Self::with_details(ErrorKind::UnAuthorized, details)
    }

    #[inline]
    pub fn bad_request(details: impl Into<String>) -> Self {
        Self::with_details(ErrorKind::BadRequest, details)
    }

    #[inline]
    pub fn code_invalid(details: impl Into<String>) -> Self {
        Self::with_details(ErrorKind::CodeIsInvalid, details)
    }

    #[inline]
    pub fn code_expired(details: impl Into<String>) -> Self {
        Self::with_details(ErrorKind::CodeIsExpired, details)
    }

    #[inline]
    pub fn db_not_found(details: impl Into<String>) -> Self {
        Self::with_details(ErrorKind::DbError, details)
    }

    #[inline]
    pub fn not_found() -> Self {
        Self::with_kind(ErrorKind::NotFound)
    }

    #[inline]
    pub fn not_found_with_details(details: impl Into<String>) -> Self {
        Self::with_details(ErrorKind::NotFound, details)
    }

    #[inline]
    pub fn account_or_pwd() -> Self {
        Self::with_kind(ErrorKind::AccountOrPassword)
    }

    #[inline]
    pub fn body_parsing(details: impl Into<String>) -> Self {
        Self::with_details(ErrorKind::BodyParsing, details)
    }

    #[inline]
    pub fn path_parsing(err: impl StdError + 'static + Send + Sync) -> Self {
        Self::new(ErrorKind::PathParsing, err.to_string(), err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.details {
            Some(details) => write!(f, "{:?}: {}", self.kind, details),
            None => write!(f, "{:?}", self.kind),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn StdError + 'static))
    }
}

impl From<Error> for Status {
    fn from(e: Error) -> Self {
        let code = match e.kind {
            ErrorKind::UnknownError => tonic::Code::Unknown,
            ErrorKind::NotFound => tonic::Code::NotFound,
            ErrorKind::AccountOrPassword => tonic::Code::Unauthenticated,
            ErrorKind::BodyParsing
            | ErrorKind::PathParsing
            | ErrorKind::UnAuthorized
            | ErrorKind::BadRequest
            | ErrorKind::CodeIsExpired
            | ErrorKind::CodeIsInvalid => tonic::Code::InvalidArgument,
            ErrorKind::OSSError
            | ErrorKind::DbError
            | ErrorKind::ConfigReadError
            | ErrorKind::ConfigParseError
            | ErrorKind::BroadCastError
            | ErrorKind::InternalServer
            | ErrorKind::ParseError
            | ErrorKind::TonicError
            | ErrorKind::MongoDbValueAccessError
            | ErrorKind::MongoDbBsonSerError
            | ErrorKind::MongoDbOperateError
            | ErrorKind::RedisError
            | ErrorKind::IOError
            | ErrorKind::ReqwestError
            | ErrorKind::BinCode
            | ErrorKind::ServiceNotFound => tonic::Code::Internal,
        };

        error!("gRPC procedure encountered error{:?}", e);
        let kind = format!("{:?}", e.kind);
        let details = e.details.unwrap_or_default();
        Status::with_details(code, &kind, details.into())
    }
}

impl From<Status> for Error {
    fn from(status: Status) -> Self {
        let kind = match status.message() {
            "UnknownError" => ErrorKind::UnknownError,
            "DbError" => ErrorKind::DbError,
            "ConfigReadError" => ErrorKind::ConfigReadError,
            "ConfigParseError" => ErrorKind::ConfigParseError,
            "NotFound" => ErrorKind::NotFound,
            "BroadCastError" => ErrorKind::BroadCastError,
            "InternalServer" => ErrorKind::InternalServer,
            "BodyParsing" => ErrorKind::BodyParsing,
            "PathParsing" => ErrorKind::PathParsing,
            "UnAuthorized" => ErrorKind::UnAuthorized,
            "ParseError" => ErrorKind::ParseError,
            "TonicError" => ErrorKind::TonicError,
            "MongoDbValueAccessError" => ErrorKind::MongoDbValueAccessError,
            "MongoDbOperateError" => ErrorKind::MongoDbOperateError,
            "MongoDbBsonSerError" => ErrorKind::MongoDbBsonSerError,
            "RedisError" => ErrorKind::RedisError,
            "IOError" => ErrorKind::IOError,
            "ReqwestError" => ErrorKind::ReqwestError,
            "ServiceNotFound" => ErrorKind::ServiceNotFound,
            "BadRequest" => ErrorKind::BadRequest,
            "AccountOrPassword" => ErrorKind::AccountOrPassword,
            "OSSError" => ErrorKind::OSSError,
            "CodeIsExpired" => ErrorKind::CodeIsExpired,
            "CodeIsInvalid" => ErrorKind::CodeIsInvalid,
            "BinCode" => ErrorKind::BinCode,
            _ => ErrorKind::UnknownError, // Default to UnknownError if the kind is not recognized
        };

        let details = String::from_utf8(status.details().to_vec()).ok();

        Self {
            kind,
            details,
            source: None,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let status_code = match self.kind {
            ErrorKind::BodyParsing
            | ErrorKind::PathParsing
            | ErrorKind::BadRequest
            | ErrorKind::CodeIsExpired
            | ErrorKind::CodeIsInvalid => StatusCode::BAD_REQUEST,
            ErrorKind::AccountOrPassword | ErrorKind::UnAuthorized => StatusCode::UNAUTHORIZED,
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            ErrorKind::DbError
            | ErrorKind::ParseError
            | ErrorKind::ConfigReadError
            | ErrorKind::ConfigParseError
            | ErrorKind::TonicError
            | ErrorKind::MongoDbValueAccessError
            | ErrorKind::MongoDbOperateError
            | ErrorKind::MongoDbBsonSerError
            | ErrorKind::RedisError
            | ErrorKind::ReqwestError
            | ErrorKind::IOError
            | ErrorKind::ServiceNotFound
            | ErrorKind::BroadCastError
            | ErrorKind::OSSError
            | ErrorKind::BinCode
            | ErrorKind::InternalServer
            | ErrorKind::UnknownError => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // todo is need to log error?
        error!("custom error to http error{:?}", self);
        (status_code, Json(self)).into_response()
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::new(ErrorKind::IOError, value.to_string(), value)
    }
}

impl From<redis::RedisError> for Error {
    fn from(value: redis::RedisError) -> Self {
        Self::new(ErrorKind::RedisError, value.to_string(), value)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(value: serde_yaml::Error) -> Self {
        Self::new(ErrorKind::ConfigParseError, value.to_string(), value)
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::new(ErrorKind::ReqwestError, value.to_string(), value)
    }
}

impl From<ValueAccessError> for Error {
    fn from(value: ValueAccessError) -> Self {
        Self::new(ErrorKind::MongoDbValueAccessError, value.to_string(), value)
    }
}

impl From<mongodb::bson::ser::Error> for Error {
    fn from(value: mongodb::bson::ser::Error) -> Self {
        Self::new(ErrorKind::MongoDbBsonSerError, value.to_string(), value)
    }
}

impl From<mongodb::error::Error> for Error {
    fn from(value: mongodb::error::Error) -> Self {
        Self::new(ErrorKind::MongoDbOperateError, value.to_string(), value)
    }
}

// convert sqlx::Error to Error::ConfilictReservation
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::new(ErrorKind::DbError, value.to_string(), value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::new(ErrorKind::ParseError, value.to_string(), value)
    }
}

impl From<tonic::transport::Error> for Error {
    fn from(value: tonic::transport::Error) -> Self {
        Self::new(ErrorKind::TonicError, value.to_string(), value)
    }
}

/// SdkError is not Send and Sync, so we just extract the details
impl<E> From<SdkError<E>> for Error
where
    E: StdError + 'static,
{
    fn from(sdk_error: SdkError<E>) -> Self {
        let kind = ErrorKind::OSSError;

        let details = sdk_error.to_string();

        Self::with_details(kind, details)
    }
}

impl From<bincode::Error> for Error {
    fn from(value: bincode::Error) -> Self {
        Self::new(ErrorKind::BinCode, value.to_string(), value)
    }
}
