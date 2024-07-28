// use axum::http::StatusCode;
// use axum::response::{IntoResponse, Response};
// use axum::Json;
// use mongodb::bson::document::ValueAccessError;
// use serde_json::json;
// use tonic::{Code, Status};
// use tracing::error;

// type Message = String;
// type Location = String;
// type Path = String;

// pub struct Error
// where
//     T: std::error::Error,
// {
//     kind: ErrorKind,
//     error: Option<T>,
//     details: Option<String>,
// }

// #[derive(Debug)]
// pub enum ErrorKind {
//     // #[error("unknown errors")]
//     UnknownError,

//     // #[error("database errors{0}")]
//     DbError/* (sqlx::Error) */,

//     // #[error("config file read errors")]
//     ConfigReadError,

//     // #[error("config parse errors")]
//     ConfigParseError/* (serde_yaml::Error) */,

//     #[error("Not found by the given condition")]
//     NotFound,

//     #[error("broadcast errors")]
//     BroadCastError,

//     // 500
//     #[error("internal server errors")]
//     InternalServer(Message),

//     // 400
//     #[error("body parsing errors")]
//     BodyParsing(Message, Path),

//     #[error("path parsing errors{0}")]
//     PathParsing(Message, Option<Location>),

//     #[error("unauthorized request{0}, path: {1}")]
//     UnAuthorized(Message, Path),

//     #[error("parse error: {0}")]
//     ParseError(Message),

//     #[error("tonic error: {0}")]
//     TonicError(Message),

//     #[error("mongodb value access error: {0}")]
//     MongoDbValueAccessError(ValueAccessError),

//     #[error("mongodb bson serde error: {0}")]
//     MongoDbBsonSerError(mongodb::bson::ser::Error),

//     #[error("mongodb value access error: {0}")]
//     MongoDbOperateError(mongodb::error::Error),

//     #[error("redis error: {0}")]
//     RedisError(redis::RedisError),

//     #[error("io error: {0}")]
//     IOError(std::io::Error),

//     #[error("reqwest error: {0}")]
//     ReqwestError(reqwest::Error),

//     #[error("service not found: {0}")]
//     ServiceNotFound(String),

//     #[error("invalid register code ")]
//     InvalidRegisterCode,

//     #[error("bad request: {0}")]
//     BadRequest(String),

//     #[error("account or password error")]
//     AccountOrPassword,
// }

// impl From<std::io::Error> for Error {
//     fn from(value: std::io::Error) -> Self {
//         Self::IOError(value)
//     }
// }

// impl From<redis::RedisError> for Error {
//     fn from(value: redis::RedisError) -> Self {
//         Self::RedisError(value)
//     }
// }

// impl From<serde_yaml::Error> for Error {
//     fn from(value: serde_yaml::Error) -> Self {
//         Self::ConfigParseError(value)
//     }
// }

// impl From<reqwest::Error> for Error {
//     fn from(value: reqwest::Error) -> Self {
//         Self::ReqwestError(value)
//     }
// }

// impl From<ValueAccessError> for Error {
//     fn from(value: ValueAccessError) -> Self {
//         Self::MongoDbValueAccessError(value)
//     }
// }

// impl From<mongodb::bson::ser::Error> for Error {
//     fn from(value: mongodb::bson::ser::Error) -> Self {
//         Self::MongoDbBsonSerError(value)
//     }
// }

// impl From<mongodb::error::Error> for Error {
//     fn from(value: mongodb::error::Error) -> Self {
//         Self::MongoDbOperateError(value)
//     }
// }

// // convert sqlx::Error to Error::ConfilictReservation
// impl From<sqlx::Error> for Error {
//     fn from(e: sqlx::Error) -> Self {
//         Error::DbError(e)
//     }
// }

// impl From<serde_json::Error> for Error {
//     fn from(value: serde_json::Error) -> Self {
//         Self::ParseError(value.to_string())
//     }
// }

// // convert abi::Error to tonic::Status
// impl From<Error> for Status {
//     fn from(e: Error) -> Self {
//         // log the error details
//         error!("{:?}", e);
//         match e {
//             Error::DbError(e) => Status::internal(e.to_string()),
//             Error::UnknownError => Status::unknown(e.to_string()),
//             Error::ConfigReadError | Error::ConfigParseError(_) => Status::internal(e.to_string()),
//             Error::NotFound => Status::not_found(e.to_string()),
//             Error::InternalServer(msg) => Status::internal(msg),
//             Error::BodyParsing(_, _) => Status::invalid_argument(e.to_string()),
//             Error::PathParsing(_, _) => Status::invalid_argument(e.to_string()),
//             Error::UnAuthorized(_, _) => Status::unauthenticated(e.to_string()),
//             Error::BroadCastError => Status::internal(e.to_string()),
//             Error::ParseError(e) => Status::internal(e.to_string()),
//             Error::TonicError(e) => Status::internal(e.to_string()),
//             Error::MongoDbValueAccessError(e) => Status::internal(e.to_string()),
//             Error::MongoDbOperateError(e) => Status::internal(e.to_string()),
//             Error::MongoDbBsonSerError(e) => Status::internal(e.to_string()),
//             Error::RedisError(_) => Status::internal(e.to_string()),
//             Error::ReqwestError(_) => Status::internal(e.to_string()),
//             Error::IOError(_) => Status::internal(e.to_string()),
//             Error::ServiceNotFound(_) => Status::internal(e.to_string()),
//             Error::InvalidRegisterCode => Status::invalid_argument(e.to_string()),
//             Error::BadRequest(_) => Status::invalid_argument(e.to_string()),
//             Error::AccountOrPassword => Status::invalid_argument(e.to_string()),
//         }
//     }
// }

// impl From<Status> for Error {
//     fn from(status: Status) -> Self {
//         let message = status.message().to_string();
//         match status.code() {
//             Code::Ok => Error::UnknownError, // 根据需要处理 Ok 状态
//             Code::Cancelled => Error::InternalServer(message),
//             Code::Unknown => Error::InternalServer(message),
//             Code::InvalidArgument => Error::BodyParsing(message, Path::default()),
//             Code::DeadlineExceeded => Error::InternalServer(message),
//             Code::NotFound => Error::NotFound,
//             Code::AlreadyExists => Error::InternalServer(message),
//             Code::PermissionDenied => Error::InternalServer(message),
//             Code::ResourceExhausted => Error::InternalServer(message),
//             Code::FailedPrecondition => Error::InternalServer(message),
//             Code::Aborted => Error::InternalServer(message),
//             Code::OutOfRange => Error::InternalServer(message),
//             Code::Unimplemented => Error::InternalServer(message),
//             Code::Internal => Error::InternalServer(message),
//             Code::Unavailable => Error::InternalServer(message),
//             Code::DataLoss => Error::InternalServer(message),
//             Code::Unauthenticated => Error::UnAuthorized(message, Path::default()),
//         }
//     }
// }

// impl IntoResponse for Error {
//     fn into_response(self) -> Response {
//         let status_code = match self {
//             Error::UnknownError => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::BodyParsing(_, _) => StatusCode::BAD_REQUEST,
//             Error::PathParsing(_, _) => StatusCode::BAD_REQUEST,
//             Error::UnAuthorized(_, _) => StatusCode::UNAUTHORIZED,
//             Error::NotFound => StatusCode::NOT_FOUND,
//             Error::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::ParseError(_) | Error::ConfigReadError => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::ConfigParseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::TonicError(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::MongoDbValueAccessError(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::MongoDbOperateError(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::MongoDbBsonSerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::RedisError(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::ReqwestError(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::IOError(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::ServiceNotFound(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::InternalServer(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::BroadCastError => StatusCode::INTERNAL_SERVER_ERROR,
//             Error::InvalidRegisterCode => StatusCode::BAD_REQUEST,
//             Error::BadRequest(_) => StatusCode::BAD_REQUEST,
//             Error::AccountOrPassword => StatusCode::UNAUTHORIZED,
//         };
//         let msg = self.to_string();

//         (status_code, Json(json!({"error": msg}))).into_response()
//     }
// }
// impl IntoResponse for Error {
//     fn into_response(self) -> Response {
//         let (status, msg) = match self {
//             Error::UnknownError => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 "UNKNOWN ERROR".to_string(),
//             ),
//             Error::BodyParsing(msg, path) => (
//                 StatusCode::BAD_REQUEST,
//                 format!("Bad Request errors: {{ message: {}, path: {}}}", msg, path),
//             ),
//             Error::PathParsing(msg, location) => {
//                 let mut msg = format!("Bad Request errors: {}", msg);
//                 if location.is_some() {
//                     msg = format!("Bad Request errors: in {} : {}", location.unwrap(), msg);
//                 }
//                 (StatusCode::BAD_REQUEST, msg)
//             }
//             Error::UnAuthorized(msg, path) => (
//                 StatusCode::UNAUTHORIZED,
//                 format!("Bad Request errors: {{ message: {}, path: {}}}", msg, path),
//             ),
//             Error::NotFound => (StatusCode::NOT_FOUND, "NOT FOUND".to_string()),
//             Error::DbError(e) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("INTERNAL SERVER ERROR {:?}", e),
//             ),
//             Error::ParseError(_) | Error::ConfigReadError => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 "INTERNAL SERVER ERROR ".to_string(),
//             ),
//             Error::ConfigParseError(e) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("INTERNAL SERVER ERROR {e}"),
//             ),
//             Error::TonicError(e) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("INTERNAL SERVER ERROR {e}"),
//             ),
//             Error::MongoDbValueAccessError(e) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("INTERNAL SERVER ERROR {:?}", e),
//             ),
//             Error::MongoDbOperateError(e) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("INTERNAL SERVER ERROR {e}"),
//             ),
//             Error::MongoDbBsonSerError(e) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("INTERNAL SERVER ERROR {e}"),
//             ),
//             Error::RedisError(e) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("INTERNAL SERVER ERROR {:?}", e),
//             ),
//             Error::ReqwestError(e) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("INTERNAL SERVER ERROR {e}"),
//             ),
//             Error::IOError(e) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("INTERNAL SERVER ERROR {e}"),
//             ),
//             Error::ServiceNotFound(e) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("INTERNAL SERVER ERROR {e}"),
//             ),
//             Error::InternalServer(e) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("INTERNAL SERVER ERROR {e}"),
//             ),
//             Error::BroadCastError => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 "BROADCAST ERROR".to_string(),
//             ),
//             Error::InvalidRegisterCode => {
//                 (StatusCode::BAD_REQUEST, "INVALID REGISTER CODE".to_string())
//             }
//             Error::BadRequest(msg) => (
//                 StatusCode::BAD_REQUEST,
//                 format!("Bad Request errors: {msg}"),
//             ),
//             Error::AccountOrPassword => (
//                 StatusCode::UNAUTHORIZED,
//                 "INVALID ACCOUNT OR PASSWORD".to_string(),
//             ),
//         };
//         error!("http request api error: {:?}", msg);
//         (status, Json(json!({"message":msg}))).into_response()
//     }
// }

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
    InvalidRegisterCode,
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
            ErrorKind::DbError => tonic::Code::Internal,
            ErrorKind::ConfigReadError => tonic::Code::Internal,
            ErrorKind::ConfigParseError => tonic::Code::Internal,
            ErrorKind::NotFound => tonic::Code::NotFound,
            ErrorKind::BroadCastError => tonic::Code::Internal,
            ErrorKind::InternalServer => tonic::Code::Internal,
            ErrorKind::BodyParsing => tonic::Code::InvalidArgument,
            ErrorKind::PathParsing => tonic::Code::InvalidArgument,
            ErrorKind::UnAuthorized => tonic::Code::Unauthenticated,
            ErrorKind::ParseError => tonic::Code::Internal,
            ErrorKind::TonicError => tonic::Code::Internal,
            ErrorKind::MongoDbValueAccessError => tonic::Code::Internal,
            ErrorKind::MongoDbBsonSerError => tonic::Code::Internal,
            ErrorKind::MongoDbOperateError => tonic::Code::Internal,
            ErrorKind::RedisError => tonic::Code::Internal,
            ErrorKind::IOError => tonic::Code::Internal,
            ErrorKind::ReqwestError => tonic::Code::Internal,
            ErrorKind::ServiceNotFound => tonic::Code::Internal,
            ErrorKind::InvalidRegisterCode => tonic::Code::InvalidArgument,
            ErrorKind::BadRequest => tonic::Code::InvalidArgument,
            ErrorKind::AccountOrPassword => tonic::Code::Unauthenticated,
            ErrorKind::OSSError => tonic::Code::Internal,
            ErrorKind::CodeIsExpired => tonic::Code::InvalidArgument,
            ErrorKind::CodeIsInvalid => tonic::Code::InvalidArgument,
            ErrorKind::BinCode => tonic::Code::Internal,
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
            "InvalidRegisterCode" => ErrorKind::InvalidRegisterCode,
            "BadRequest" => ErrorKind::BadRequest,
            "AccountOrPassword" => ErrorKind::AccountOrPassword,
            "OSSError" => ErrorKind::OSSError,
            "CodeIsExpired" => ErrorKind::CodeIsExpired,
            "CodeIsInvalid" => ErrorKind::CodeIsInvalid,
            "BinCode" => ErrorKind::BinCode,
            _ => ErrorKind::UnknownError, // Default to UnknownError if the kind is not recognized
        };

        let details = match String::from_utf8(status.details().to_vec()) {
            Ok(details) => Some(details),
            Err(_) => None,
        };

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
            ErrorKind::UnknownError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::BodyParsing => StatusCode::BAD_REQUEST,
            ErrorKind::PathParsing => StatusCode::BAD_REQUEST,
            ErrorKind::UnAuthorized => StatusCode::UNAUTHORIZED,
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            ErrorKind::DbError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::ParseError | ErrorKind::ConfigReadError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::ConfigParseError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::TonicError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::MongoDbValueAccessError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::MongoDbOperateError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::MongoDbBsonSerError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::RedisError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::ReqwestError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::IOError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::ServiceNotFound => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::InternalServer => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::BroadCastError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::InvalidRegisterCode => StatusCode::BAD_REQUEST,
            ErrorKind::BadRequest => StatusCode::BAD_REQUEST,
            ErrorKind::AccountOrPassword => StatusCode::UNAUTHORIZED,
            ErrorKind::OSSError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::CodeIsExpired => StatusCode::BAD_REQUEST,
            ErrorKind::CodeIsInvalid => StatusCode::BAD_REQUEST,
            ErrorKind::BinCode => StatusCode::INTERNAL_SERVER_ERROR,
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
