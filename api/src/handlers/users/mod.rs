use std::net::{IpAddr, SocketAddr};

use axum::Json;
use base64::prelude::*;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

use abi::errors::Error;
use abi::message::User;

mod oauth2;
mod user_handlers;

pub use oauth2::*;
use tracing::error;
pub use user_handlers::*;
use xdb::search_by_ip;

use crate::AppState;
use crate::api_utils::ip_region::parse_region;

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
            return Err(Error::bad_request("parameter is none"));
        }
        let pwd = BASE64_STANDARD_NO_PAD
            .decode(&self.password)
            .map_err(Error::internal)?;
        self.password = String::from_utf8(pwd).map_err(Error::internal)?;
        Ok(())
    }
}

pub const REFRESH_EXPIRES: i64 = 24 * 60 * 60;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
}

const EXPIRES: i64 = 60 * 60 * 4;

impl Claims {
    pub fn new(sub: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        let exp = now + EXPIRES;
        Self { sub, exp, iat: now }
    }
}

pub async fn gen_token(
    app_state: &AppState,
    mut user: User,
    addr: SocketAddr,
) -> Result<Json<Token>, Error> {
    // generate token
    let mut claims = Claims::new(user.id.clone());

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(app_state.jwt_secret.as_bytes()),
    )
    .map_err(Error::internal)?;

    claims.exp += REFRESH_EXPIRES;
    let refresh_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(app_state.jwt_secret.as_bytes()),
    )
    .map_err(Error::internal)?;

    app_state.cache.user_login(&user.account).await?;

    // get websocket service address
    // let ws_lb = Arc::get_mut(&mut app_state.ws_lb).unwrap();
    let ws_addr = app_state
        .ws_lb
        .get_service()
        .await
        .map(|addr| format!("{}://{}/ws", &app_state.ws_config.protocol, addr))
        .ok_or(Error::internal_with_details(
            "No websocket service available",
        ))?;

    // query region
    user.region = match addr.ip() {
        IpAddr::V4(ip) => match search_by_ip(ip) {
            Ok(region) => parse_region(&region),
            Err(e) => {
                error!("search region error: {:?}", e);
                None
            }
        },
        IpAddr::V6(_) => None,
    };

    if user.region.is_some() {
        app_state
            .db
            .user
            .update_region(&user.id, user.region.as_ref().unwrap())
            .await?;
    }

    Ok(Json(Token {
        user,
        token,
        refresh_token,
        ws_addr,
    }))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModifyPwdRequest {
    pub user_id: String,
    pub email: String,
    pub pwd: String,
    pub code: String,
}

impl ModifyPwdRequest {
    pub fn decode(&mut self) -> Result<(), Error> {
        // base64 decode
        if self.user_id.is_empty()
            || self.email.is_empty()
            || self.pwd.is_empty()
            || self.code.is_empty()
        {
            return Err(Error::bad_request("parameter is none"));
        }
        let pwd = BASE64_STANDARD_NO_PAD
            .decode(&self.pwd)
            .map_err(Error::internal)?;
        self.pwd = String::from_utf8(pwd).map_err(Error::internal)?;
        Ok(())
    }
}
