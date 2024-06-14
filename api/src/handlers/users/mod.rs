use base64::prelude::*;
use serde::{Deserialize, Serialize};

use abi::errors::Error;
use abi::message::User;

mod user_handlers;

pub use user_handlers::*;

// 定义request model
#[derive(Debug, Deserialize, Serialize)]
pub struct UserRegister {
    pub avatar: String,
    pub name: String,
    pub password: String,
    pub email: String,
    pub code: String,
}

#[derive(Serialize)]
pub struct Token {
    user: User,
    token: String,
    refresh_token: String,
    ws_addr: String,
}

#[derive(Deserialize, Debug)]
pub struct LoginRequest {
    pub account: String,
    pub password: String,
}

impl LoginRequest {
    pub fn decode(&mut self) -> Result<(), Error> {
        // base64 decode
        if self.account.is_empty() || self.password.is_empty() {
            return Err(Error::BadRequest("parameter is none".to_string()));
        }
        let pwd = BASE64_STANDARD
            .decode(&self.password)
            .map_err(|e| Error::InternalServer(e.to_string()))?;
        self.password = String::from_utf8(pwd).map_err(|e| Error::InternalServer(e.to_string()))?;
        Ok(())
    }
}

pub const REFRESH_EXPIRES: u64 = 24;
// pub const REFRESH_EXPIRES: u64 = 24 * 60 * 60;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
}

const EXPIRES: u64 = 10;

impl Claims {
    pub fn new(sub: String) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        let exp = now + EXPIRES;
        Self { sub, exp, iat: now }
    }
}
