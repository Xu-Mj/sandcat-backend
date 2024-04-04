use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use mongodb::bson::document::ValueAccessError;
use serde_json::json;

type Message = String;
type Location = String;
type Path = String;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unknown errors")]
    UnknownError,

    #[error("database errors{0}")]
    DbError(sqlx::Error),

    #[error("config file read errors")]
    ConfigReadError,

    #[error("config parse errors")]
    ConfigParseError(serde_yaml::Error),

    #[error("not found")]
    NotFound,

    #[error("broadcast errors")]
    BroadCastError,

    // 500
    #[error("internal server errors")]
    InternalServer(Message),

    // 400
    #[error("body parsing errors")]
    BodyParsing(Message, Path),

    #[error("path parsing errors{0}")]
    PathParsing(Message, Option<Location>),

    #[error("unauthorized request{0}, path: {1}")]
    UnAuthorized(Message, Path),

    #[error("parse error: {0}")]
    ParseError(Message),

    #[error("rpc error: {0}")]
    TonicError(Message),

    #[error("mongodb value access error: {0}")]
    MongoDbValueAccessError(ValueAccessError),

    #[error("mongodb value access error: {0}")]
    MongoDbBsonSerError(mongodb::bson::ser::Error),

    #[error("mongodb value access error: {0}")]
    MongoDbOperateError(mongodb::error::Error),

    #[error("redis error: {0}")]
    RedisError(redis::RedisError),

    #[error("reqwest error: {0}")]
    IOError(std::io::Error),

    #[error("reqwest error: {0}")]
    ReqwestError(reqwest::Error),

    #[error("SERVICE NOT FOUND: {0}")]
    ServiceNotFound(String),

    #[error("invalid register code ")]
    InvalidRegisterCode,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("account or password error")]
    AccountOrPassword,
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value)
    }
}

impl From<redis::RedisError> for Error {
    fn from(value: redis::RedisError) -> Self {
        Self::RedisError(value)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(value: serde_yaml::Error) -> Self {
        Self::ConfigParseError(value)
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value)
    }
}

impl From<ValueAccessError> for Error {
    fn from(value: ValueAccessError) -> Self {
        Self::MongoDbValueAccessError(value)
    }
}

impl From<mongodb::bson::ser::Error> for Error {
    fn from(value: mongodb::bson::ser::Error) -> Self {
        Self::MongoDbBsonSerError(value)
    }
}

impl From<mongodb::error::Error> for Error {
    fn from(value: mongodb::error::Error) -> Self {
        Self::MongoDbOperateError(value)
    }
}

// convert sqlx::Error to Error::ConfilictReservation
impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::Database(e) => Error::DbError(sqlx::Error::Database(e)),
            sqlx::Error::RowNotFound => Error::NotFound,
            _ => Error::DbError(e),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::ParseError(value.to_string())
    }
}
// rpc PartialEq for Error {
//     fn eq(&self, other: &Self) -> bool {
//         match (self, other) {
//             (Self::DbError(_), Self::DbError(_)) => true,
//             _ => core::mem::discriminant(self) == core::mem::discriminant(other),
//         }
//     }
// }

// convert abi::Error to tonic::Status
impl From<Error> for tonic::Status {
    fn from(e: Error) -> Self {
        match e {
            Error::DbError(e) => tonic::Status::internal(format!("DB errors: {e}")),

            Error::UnknownError => tonic::Status::unknown("Unknown errors"),

            Error::ConfigReadError | Error::ConfigParseError(_) => {
                tonic::Status::internal(e.to_string())
            }
            Error::NotFound => {
                tonic::Status::not_found("No reservation found by the given condition")
            }
            Error::InternalServer(msg) => tonic::Status::internal(msg),
            Error::BodyParsing(_, _) => tonic::Status::invalid_argument(e.to_string()),
            Error::PathParsing(_, _) => tonic::Status::invalid_argument(e.to_string()),
            Error::UnAuthorized(_, _) => tonic::Status::unauthenticated(e.to_string()),
            Error::BroadCastError => tonic::Status::internal("BROADCAST ERROR"),
            Error::ParseError(e) => tonic::Status::internal(format!("PARSE ERROR: {e}")),
            Error::TonicError(e) => tonic::Status::internal(format!("TONIC ERROR: {e}")),
            Error::MongoDbValueAccessError(e) => {
                tonic::Status::internal(format!("MONGODB VALUE ACCESS ERROR: {e}"))
            }
            Error::MongoDbOperateError(e) => {
                tonic::Status::internal(format!("MONGODB OPERATE ERROR: {e}"))
            }
            Error::MongoDbBsonSerError(e) => {
                tonic::Status::internal(format!("MONGODB BSON SER ERROR: {e}"))
            }
            Error::RedisError(_) => tonic::Status::internal(format!("REDIS ERROR: {e}")),
            Error::ReqwestError(_) => tonic::Status::internal(format!("REDIS ERROR: {e}")),
            Error::IOError(_) => tonic::Status::internal(format!("IO ERROR: {e}")),
            Error::ServiceNotFound(_) => tonic::Status::internal(format!("SERVICE NOT FOUND: {e}")),
            Error::InvalidRegisterCode => {
                tonic::Status::invalid_argument(format!("REGISTER ERROR: {e}"))
            }
            Error::BadRequest(_) => tonic::Status::invalid_argument(format!(" BAD REQUEST: {e}")),
            Error::AccountOrPassword => {
                tonic::Status::invalid_argument(" INVALID ACCOUNT OR PASSWORD".to_string())
            }
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            Error::UnknownError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "UNKNOWN ERROR".to_string(),
            ),
            Error::BodyParsing(msg, path) => (
                StatusCode::BAD_REQUEST,
                format!("Bad Request errors: {{ message: {}, path: {}}}", msg, path),
            ),
            Error::PathParsing(msg, location) => {
                let mut msg = format!("Bad Request errors: {}", msg);
                if location.is_some() {
                    msg = format!("Bad Request errors: in {} : {}", location.unwrap(), msg);
                }
                (StatusCode::BAD_REQUEST, msg)
            }
            Error::UnAuthorized(msg, path) => (
                StatusCode::UNAUTHORIZED,
                format!("Bad Request errors: {{ message: {}, path: {}}}", msg, path),
            ),
            Error::NotFound => (StatusCode::NOT_FOUND, "NOT FOUND".to_string()),
            Error::DbError(_)
            | Error::ConfigReadError
            | Error::ConfigParseError(_)
            | Error::ParseError(_)
            | Error::TonicError(_)
            | Error::MongoDbValueAccessError(_)
            | Error::MongoDbOperateError(_)
            | Error::MongoDbBsonSerError(_)
            | Error::RedisError(_)
            | Error::ReqwestError(_)
            | Error::IOError(_)
            | Error::ServiceNotFound(_)
            | Error::InternalServer(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL SERVER ERROR ".to_string(),
            ),
            Error::BroadCastError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "BROADCAST ERROR".to_string(),
            ),
            Error::InvalidRegisterCode => {
                (StatusCode::BAD_REQUEST, "INVALID REGISTER CODE".to_string())
            }
            Error::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                format!("Bad Request errors: {msg}"),
            ),
            Error::AccountOrPassword => (
                StatusCode::UNAUTHORIZED,
                "INVALID ACCOUNT OR PASSWORD".to_string(),
            ),
        };
        (status, Json(json!({"message":msg}))).into_response()
    }
}
