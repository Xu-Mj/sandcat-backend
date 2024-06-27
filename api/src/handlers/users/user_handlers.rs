use axum::extract::{ConnectInfo, State};
use axum::Json;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use lettre::message::header::ContentType;
use lettre::message::{MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use nanoid::nanoid;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use tera::{Context, Tera};
use tracing::{debug, error, info};
use xdb::search_by_ip;

use abi::errors::Error;
use abi::message::{
    CreateUserRequest, GetUserRequest, SearchUserRequest, UpdateRegionRequest, UpdateUserRequest,
    User, UserUpdate, UserWithMatchType, VerifyPwdRequest,
};

use crate::api_utils::custom_extract::{JsonExtractor, PathExtractor, PathWithAuthExtractor};
use crate::api_utils::ip_region::parse_region;
use crate::handlers::users::{Claims, LoginRequest, Token, UserRegister};
use crate::AppState;

use super::REFRESH_EXPIRES;

/// refresh auth token
pub async fn refresh_token(
    State(app_state): State<AppState>,
    PathExtractor((token, is_refresh)): PathExtractor<(String, bool)>,
) -> Result<String, Error> {
    let claim = match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(app_state.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(err) => {
            debug!("token is expired");
            return Err(Error::UnAuthorized(
                format!("UnAuthorized Request: {:?}", err),
                "/refresh_token".to_string(),
            ));
        }
    };
    let mut claims = Claims::new(claim.claims.sub.clone());
    if is_refresh {
        claims.exp += REFRESH_EXPIRES;
    }
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(app_state.jwt_secret.as_bytes()),
    )
    .map_err(|err| Error::InternalServer(err.to_string()))?;
    Ok(token)
}

/// register new user
pub async fn create_user(
    State(app_state): State<AppState>,
    JsonExtractor(new_user): JsonExtractor<UserRegister>,
) -> Result<Json<User>, Error> {
    // verify register code
    app_state
        .cache
        .get_register_code(&new_user.email)
        .await?
        .filter(|code| *code == new_user.code)
        .ok_or_else(|| Error::InvalidRegisterCode)?;

    // encode the password
    let salt = utils::generate_salt();
    let password = utils::hash_password(new_user.password.as_bytes(), &salt)?;

    let id = nanoid!();
    // convert user to db user
    let user2db = User {
        id: id.clone(),
        name: new_user.name,
        account: id,
        password,
        salt,
        email: Some(new_user.email.clone()),
        avatar: new_user.avatar,
        ..Default::default()
    };

    // todo need to check the email is registered already
    let request = CreateUserRequest {
        user: Some(user2db),
    };
    let mut db_rpc = app_state.db_rpc.clone();
    let response = db_rpc.create_user(request).await.map_err(|err| {
        error!("create user error: {:?}", err);
        Error::InternalServer(err.message().to_string())
    })?;

    let user = response
        .into_inner()
        .user
        .ok_or_else(|| Error::InternalServer("Unknown Error".to_string()))?;

    // delete register code from cache
    app_state.cache.del_register_code(&new_user.email).await?;

    Ok(Json(user))
}

pub async fn update_user(
    State(app_state): State<AppState>,
    JsonExtractor(user): JsonExtractor<UserUpdate>,
) -> Result<Json<User>, Error> {
    // todo need to check the email is registered already
    let request = UpdateUserRequest { user: Some(user) };
    let mut db_rpc = app_state.db_rpc.clone();
    let response = db_rpc.update_user(request).await.map_err(|err| {
        error!("create user error: {:?}", err);
        Error::InternalServer(err.message().to_string())
    })?;

    let user = response
        .into_inner()
        .user
        .ok_or_else(|| Error::InternalServer("Unknown Error".to_string()))?;

    Ok(Json(user))
}

pub async fn get_user_by_id(
    State(app_state): State<AppState>,
    PathExtractor(id): PathExtractor<String>,
) -> Result<Json<User>, Error> {
    let mut db_rpc = app_state.db_rpc.clone();
    let request = GetUserRequest { user_id: id };
    let user = db_rpc
        .get_user(request)
        .await
        .map_err(|err| Error::InternalServer(err.message().to_string()))?
        .into_inner()
        .user
        .ok_or_else(|| Error::NotFound)?;
    Ok(Json(user))
}

pub async fn search_user(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, pattern)): PathWithAuthExtractor<(String, String)>,
) -> Result<Json<Option<UserWithMatchType>>, Error> {
    let mut db_rpc = app_state.db_rpc.clone();
    let request = SearchUserRequest { user_id, pattern };
    let user = db_rpc
        .search_user(request)
        .await
        .map_err(|err| Error::InternalServer(err.message().to_string()))?
        .into_inner()
        .user;
    Ok(Json(user))
}

pub async fn logout(
    State(app_state): State<AppState>,
    PathExtractor(uuid): PathExtractor<String>,
) -> Result<(), Error> {
    app_state.cache.user_logout(&uuid).await?;
    Ok(())
}

pub async fn login(
    State(app_state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    JsonExtractor(mut login): JsonExtractor<LoginRequest>,
) -> Result<Json<Token>, Error> {
    // decode password from base64
    login.decode()?;

    let mut db_rpc = app_state.db_rpc.clone();
    let mut user = db_rpc
        .verify_password(VerifyPwdRequest {
            account: login.account,
            password: login.password,
        })
        .await
        .map_err(|err| Error::InternalServer(err.message().to_string()))?
        .into_inner()
        .user
        .ok_or_else(|| Error::AccountOrPassword)?;

    // generate token
    let mut claims = Claims::new(user.name.clone());

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(app_state.jwt_secret.as_bytes()),
    )
    .map_err(|err| Error::InternalServer(err.to_string()))?;
    info!("login success token: {:?}", claims);
    claims.exp += REFRESH_EXPIRES;
    let refresh_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(app_state.jwt_secret.as_bytes()),
    )
    .map_err(|err| Error::InternalServer(err.to_string()))?;

    info!("login success token: {}", token);
    info!("login success refresh: {}", refresh_token);
    app_state.cache.user_login(&user.account).await?;

    // get websocket service address
    // let ws_lb = Arc::get_mut(&mut app_state.ws_lb).unwrap();
    let ws_addr = if let Some(addr) = app_state.ws_lb.get_service().await {
        format!("{}://{}/ws", &app_state.ws_config.protocol, addr)
    } else {
        return Err(Error::InternalServer(
            "No websocket service available".to_string(),
        ));
    };

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
        // update user region
        let request = UpdateRegionRequest {
            user_id: user.id.clone(),
            region: user.region.as_ref().unwrap().clone(),
        };
        let _ = db_rpc
            .update_user_region(request)
            .await
            .map_err(|err| Error::InternalServer(err.message().to_string()))?;
    }
    Ok(Json(Token {
        user,
        token,
        refresh_token,
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
) -> Result<(), Error> {
    if email.email.is_empty() {
        return Err(Error::BadRequest("parameter is none".to_string()));
    }

    // get email template engine tera
    let tera = Tera::new(&state.mail_config.temp_path)
        .map_err(|e| Error::InternalServer(e.to_string()))?;
    // generate random number(validate code)
    let mut rng = rand::thread_rng();
    let num: u32 = rng.gen_range(100_000..1_000_000);

    // render email template
    let mut context = Context::new();
    context.insert("numbers", &num.to_string());
    let content = tera
        .render(&state.mail_config.temp_file, &context)
        .map_err(|e| Error::InternalServer(e.to_string()))?;

    // save it to redis; expire time 5 minutes
    let msg = Message::builder()
        .from(
            state
                .mail_config
                .account
                .parse()
                .map_err(|_| Error::InternalServer("email parse failed".to_string()))?,
        )
        .to(email
            .email
            .parse()
            .map_err(|_| Error::InternalServer("user email parse failed".to_string()))?)
        .subject("Verify Login Code")
        .header(ContentType::TEXT_PLAIN)
        .multipart(
            MultiPart::alternative().singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(content),
            ),
        )
        .map_err(|err| Error::InternalServer(err.to_string()))?;

    let creds = Credentials::new(state.mail_config.account, state.mail_config.password);

    // Open a remote connection to mail
    let mailer = SmtpTransport::relay(&state.mail_config.server)
        .map_err(|err| Error::InternalServer(err.to_string()))?
        .credentials(creds)
        .build();

    // Send the email
    mailer
        .send(&msg)
        .map_err(|e| Error::InternalServer(e.to_string()))?;
    tokio::spawn(async move {
        if let Err(e) = state
            .cache
            .save_register_code(&email.email, &num.to_string())
            .await
        {
            error!("{:?}", e);
        }
        debug!("verification code：{:?}, email: {:?}", num, &email.email);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use tera::{Context, Tera};

    #[test]
    fn test_template() {
        let tera = Tera::new("fixtures/templates/*").unwrap();
        let mut rng = rand::thread_rng();
        let num: u32 = rng.gen_range(100_000..1_000_000);
        let mut context = Context::new();
        context.insert("numbers", &num.to_string());
        let string = tera.render("email_temp.html", &context).unwrap();
        println!("{}", string);
    }
}
