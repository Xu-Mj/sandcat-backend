mod conflict;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub use self::conflict::ConflictReservationInfo;

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
    ConfigParseError,

    #[error("reservation not found by given condition")]
    NotFound,

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

// impl PartialEq for Error {
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

            Error::ConfigReadError | Error::ConfigParseError => {
                tonic::Status::internal(e.to_string())
            }
            Error::NotFound => {
                tonic::Status::not_found("No reservation found by the given condition")
            }
            Error::InternalServer(msg) => tonic::Status::internal(msg),
            Error::BodyParsing(_, _) => tonic::Status::invalid_argument(e.to_string()),
            Error::PathParsing(_, _) => tonic::Status::invalid_argument(e.to_string()),
            Error::UnAuthorized(_, _) => tonic::Status::unauthenticated(e.to_string()),
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
            | Error::ConfigParseError
            | Error::InternalServer(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "UNKNOWN ERROR".to_string(),
            ),
        };
        (status, Json(json!({"message":msg}))).into_response()
    }
}
