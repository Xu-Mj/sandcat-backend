use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use diesel::{Queryable, Selectable};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::infra::db::schema::users;
use crate::infra::errors::InfraError;

// todo add region attribute
#[derive(Clone, Serialize, Default, Deserialize, Selectable, Queryable, Debug)]
#[diesel(table_name = users)]
// 开启编译期字段检查，主要检查字段类型、数量是否匹配，可选
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: String,
    pub name: String,
    pub account: String,
    #[serde(skip)]
    pub password: String,
    pub avatar: String,
    pub gender: String,
    pub age: i32,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub birthday: Option<chrono::NaiveDateTime>,
    pub create_time: chrono::NaiveDateTime,
    pub update_time: chrono::NaiveDateTime,
    #[serde(skip)]
    pub is_delete: bool,
}
#[derive(Clone, Serialize, Default, Deserialize, Selectable, Queryable, Debug)]
#[diesel(table_name = users)]
pub struct UserView {
    pub id: String,
    pub name: String,
    pub account: String,
    pub avatar: String,
    pub gender: String,
    pub age: i32,
}

type ID = String;

#[derive(Debug)]
pub enum UserError {
    InternalServerError(String),
    NotFound(ID),
    LoginError,
    InfraError(InfraError),
    Register(RegisterErrState),
}

#[derive(Debug)]
pub enum RegisterErrState {
    CodeErr,
}

// 将用户错误转为axum响应
impl IntoResponse for UserError {
    fn into_response(self) -> Response {
        let (status, err_msg) = match self {
            UserError::InternalServerError(err_msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal Server Error: {}", err_msg),
            ),
            UserError::NotFound(id) => (StatusCode::NOT_FOUND, format!(" User {} Not Found", id)),
            UserError::InfraError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal Server Error: {}", err),
            ),
            UserError::LoginError => (
                StatusCode::FORBIDDEN,
                String::from("Account Or Password Error"),
            ),
            UserError::Register(state) => match state {
                RegisterErrState::CodeErr => (StatusCode::BAD_REQUEST, String::from("CODE ERROR")),
            },
        };
        (
            status,
            Json(json!({"resource":"UserModel","message":err_msg})),
        )
            .into_response()
    }
}
