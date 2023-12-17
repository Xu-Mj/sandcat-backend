pub use crate::handlers::users::user_handlers::{create_user, get_user_by_id};
pub(crate) mod user_handlers;


use serde::{Deserialize, Serialize};

// 定义request model
#[derive(Debug, Deserialize, Serialize)]
pub struct UserRequest {
    pub name: String,
    pub avatar: String,
    pub gender: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub birthday: Option<chrono::NaiveDateTime>,
}
