use axum::extract::State;
use axum::Json;
use jsonwebtoken::{encode, EncodingKey, Header};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use nanoid::nanoid;
use rand::Rng;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::config::CONFIG;
use crate::domain::model::user::{RegisterErrState, User, UserError};
use crate::handlers::users::UserRegister;
use crate::handlers::ws::register_ws;
use crate::infra::errors::InfraError;
use crate::infra::repositories::user_repo::{get, insert, search, verify_pwd, NewUserDb};
use crate::utils::redis::redis_crud;
use crate::utils::{JsonExtractor, PathExtractor, PathWithAuthExtractor};
use crate::AppState;

pub async fn create_user(
    State(app_state): State<AppState>,
    JsonExtractor(new_user): JsonExtractor<UserRegister>,
) -> Result<Json<User>, UserError> {
    let mut redis_conn = app_state
        .redis
        .get_async_connection()
        .await
        .map_err(|err| UserError::InternalServerError(err.to_string()))?;

    // 查询验证码

    let result: Option<String> = redis_conn
        .get(&new_user.email)
        .await
        .map_err(|err| UserError::InternalServerError(err.to_string()))?;
    if let Some(code) = result {
        if code != new_user.code {
            return Err(UserError::Register(RegisterErrState::CodeErr));
        }
    } else {
        return Err(UserError::Register(RegisterErrState::CodeErr));
    }
    // 结构体转换
    let user2db = NewUserDb {
        id: nanoid!(),
        name: new_user.name,
        account: nanoid!(),
        password: new_user.password,
        email: Some(new_user.email),
        avatar: new_user.avatar,
        ..Default::default()
    };
    // repo方法调用
    let user = insert(&app_state.pool, user2db)
        .await
        .map_err(UserError::InfraError)?;
    // 结果返回
    Ok(Json(user))
}

pub async fn get_user_by_id(
    State(app_state): State<AppState>,
    PathExtractor(id): PathExtractor<String>,
) -> Result<Json<User>, UserError> {
    let user = get(&app_state.pool, id.clone())
        .await
        .map_err(|err| match err {
            InfraError::InternalServerError(e) => UserError::InternalServerError(e),
            InfraError::NotFound => UserError::NotFound(id),
            _ => UserError::InternalServerError("Unknown Error".to_string()),
        })?;
    Ok(Json(user))
}

pub async fn search_user(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, pattern)): PathWithAuthExtractor<(String, String)>,
) -> Result<Json<Vec<User>>, UserError> {
    let users = search(&app_state.pool, user_id, pattern)
        .await
        .map_err(|err| {
            error!("search user error:{:?}", err);
            err
        })
        .unwrap_or_else(|_| vec![]);
    Ok(Json(users))
}

pub async fn logout(
    State(app_state): State<AppState>,
    PathExtractor(uuid): PathExtractor<String>,
) -> Result<(), UserError> {
    let conn = app_state
        .redis
        .get_async_connection()
        .await
        .map_err(|err| UserError::InternalServerError(err.to_string()))?;
    redis_crud::del(conn, uuid)
        .await
        .map_err(|err| UserError::InternalServerError(err.to_string()))?;
    tracing::debug!("user logout");
    Ok(())
}

#[derive(Serialize)]
pub struct Token {
    id: String,
    user: User,
    token: String,
    ws_addr: String,
}

#[derive(Deserialize, Debug)]
pub struct LoginRequest {
    pub account: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub update: u64,
}

impl Claims {
    pub fn new(sub: String) -> Self {
        let now = chrono::Local::now().timestamp_millis() as u64;
        let exp = now + 10_000;
        Self {
            sub,
            exp,
            iat: now,
            update: 10_000,
        }
    }
}

pub async fn login(
    State(app_state): State<AppState>,
    JsonExtractor(login): JsonExtractor<LoginRequest>,
) -> Result<Json<Token>, UserError> {
    let user = verify_pwd(&app_state.pool, login.account, login.password)
        .await
        .map_err(|err| {
            tracing::error!("login error: {:?}", &err);
            match err {
                InfraError::InternalServerError(e) => UserError::InternalServerError(e),
                _ => UserError::LoginError,
            }
        })?;
    // 生成token
    let claims = Claims::new(user.name.clone());

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(CONFIG.get().unwrap().jwt_secret().as_bytes()),
    )
    .map_err(|err| UserError::InternalServerError(err.to_string()))?;
    let ws_addr = register_ws(app_state.redis, user.id.clone())
        .await
        .map_err(|err| UserError::InternalServerError(err.to_string()))?;
    Ok(Json(Token {
        id: user.account.clone(),
        user,
        token,
        ws_addr,
    }))
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Email {
    pub email: String,
}

pub async fn send_email(
    State(state): State<AppState>,
    JsonExtractor(email): JsonExtractor<Email>,
) -> Result<(), UserError> {
    if email.email.is_empty() {
        return Err(UserError::InternalServerError(
            "parameter is none".to_string(),
        ));
    }
    let mut redis_conn = state
        .redis
        .get_async_connection()
        .await
        .map_err(|err| UserError::InternalServerError(err.to_string()))?;
    // 生成随机数（验证码）
    let mut rng = rand::thread_rng();
    let num: u32 = rng.gen_range(100_000..1_000_000);
    // 将随机数存入redis--五分钟有效
    let e = email.email.clone();
    tokio::spawn(async move {
        let _: () = redis_conn
            .set_ex(&e, num, 60 * 5)
            .await
            .map_err(|err| UserError::InternalServerError(err.to_string()))
            .unwrap();
        tracing::debug!("验证码：{:?}, 邮箱: {:?}", num, &e);
    });

    let email = Message::builder()
        .from(
            "653609824@qq.com"
                .parse()
                .map_err(|_| UserError::InternalServerError("邮箱parse失败".to_string()))?,
        )
        .to(email
            .email
            .parse()
            .map_err(|_| UserError::InternalServerError("邮箱parse失败".to_string()))?)
        .subject("验证登录")
        .header(ContentType::TEXT_PLAIN)
        .body(num.to_string())
        .map_err(|err| UserError::InternalServerError(err.to_string()))?;

    let creds = Credentials::new("653609824@qq.com".to_owned(), "rxkhmcpjgigsbegi".to_owned());

    // Open a remote connection to gmail
    let mailer = SmtpTransport::relay("smtp.qq.com")
        .map_err(|err| UserError::InternalServerError(err.to_string()))?
        .credentials(creds)
        .build();

    // Send the email
    mailer
        .send(&email)
        .map_err(|e| UserError::InternalServerError(e.to_string()))?;
    Ok(())
}
