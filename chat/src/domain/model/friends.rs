use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use diesel::{Associations, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::PgRow;
use sqlx::{Error, FromRow, Row};

use crate::domain::model::friend_request_status::FriendStatus;
use crate::domain::model::user::User;
use crate::infra::db::schema::friends;

type ID = String;

pub enum FriendError {
    InternalServerError(String),
    Parameter(String),
    NotFound(ID),
    // InfraError(InfraError),
}

// 将用户错误转为axum响应
impl IntoResponse for FriendError {
    fn into_response(self) -> Response {
        let (status, err_msg) = match self {
            FriendError::InternalServerError(err_msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal Server Error: {}", err_msg),
            ),
            FriendError::NotFound(id) => (StatusCode::NOT_FOUND, format!(" User {} Not Found", id)),
            // FriendError::InfraError(err) => (
            //     StatusCode::INTERNAL_SERVER_ERROR,
            //     format!("Internal Server Error: {}", err),
            // ),
            FriendError::Parameter(msg) => {
                (StatusCode::BAD_REQUEST, format!("Parameter Error: {}", msg))
            }
        };
        (
            status,
            Json(json!({"resource":"FriendModel","message":err_msg})),
        )
            .into_response()
    }
}

#[derive(Serialize, Queryable, Selectable, Associations, Debug, Insertable)]
#[diesel(table_name = friends)]
#[diesel(belongs_to(User))]
// 开启编译期字段检查，主要检查字段类型、数量是否匹配，可选
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FriendDb {
    pub id: String,
    pub friendship_id: String,
    pub user_id: String,
    pub friend_id: String,
    // 0: delete; 1: friend; 2: blacklist
    pub status: FriendStatus,
    pub remark: Option<String>,
    pub hello: Option<String>,
    pub source: Option<String>,
    pub create_time: chrono::NaiveDateTime,
    pub update_time: chrono::NaiveDateTime,
}

#[derive(Serialize, Queryable, Deserialize, Clone, Debug)]
pub struct FriendWithUser {
    pub id: String,
    pub friend_id: String,
    pub remark: Option<String>,
    pub hello: Option<String>,
    pub status: FriendStatus,
    pub create_time: chrono::NaiveDateTime,
    pub update_time: chrono::NaiveDateTime,
    pub from: Option<String>,
    pub name: String,
    pub account: String,
    pub avatar: String,
    pub gender: String,
    pub age: i32,
    /// TODO 敏感信息需要处理
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub birthday: Option<chrono::NaiveDateTime>,
}

impl FromRow<'_, PgRow> for FriendWithUser {
    fn from_row(row: &'_ PgRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.get("id"),
            friend_id: row.get("friend_id"),
            remark: row.get("remark"),
            hello: row.get("hello"),
            status: row.get("status"),
            create_time: row.get("create_time"),
            update_time: row.get("update_time"),
            from: row.get("from"),
            name: row.get("name"),
            account: row.get("account"),
            avatar: row.get("avatar"),
            gender: row.get("gender"),
            age: row.get("age"),
            phone: row.get("phone"),
            email: row.get("email"),
            address: row.get("address"),
            birthday: row.get("birthday"),
        })
    }
}
