pub use crate::handlers::users::user_handlers::{create_user, get_user_by_id, login, send_email};
pub(crate) mod user_handlers;


use serde::{Deserialize, Serialize};

// 定义request model
#[derive(Debug, Deserialize, Serialize)]
pub struct UserRegister {
    pub avatar: String,
    pub name: String,
    pub password: String,
    pub email: String,
    pub code: String,
}
