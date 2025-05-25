use std::net::SocketAddr;

use axum::Json;
use axum::extract::{ConnectInfo, State};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use lettre::message::header::ContentType;
use lettre::message::{MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use nanoid::nanoid;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};
use tracing::{debug, error};

use abi::errors::Error;
use abi::message::{User, UserUpdate, UserWithMatchType};

use crate::AppState;
use crate::api_utils::custom_extract::{JsonExtractor, PathExtractor, PathWithAuthExtractor};
use crate::handlers::users::{Claims, LoginRequest, Token, UserRegister};

use super::{ModifyPwdRequest, REFRESH_EXPIRES, gen_token};

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
            return Err(Error::unauthorized(err, "/refresh_token"));
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
    .map_err(Error::internal)?;
    Ok(token)
}

/// register new user
pub async fn create_user(
    State(app_state): State<AppState>,
    JsonExtractor(new_user): JsonExtractor<UserRegister>,
) -> Result<Json<User>, Error> {
    // verify register code
    let code = app_state
        .cache
        .get_register_code(&new_user.email)
        .await?
        .ok_or(Error::code_expired("in register"))?;
    if code != new_user.code {
        return Err(Error::code_invalid("in register"));
    }

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
    let user = app_state.db.user.create_user(user2db).await?;

    // delete register code from cache
    app_state.cache.del_register_code(&new_user.email).await?;

    Ok(Json(user))
}

pub async fn update_user(
    State(app_state): State<AppState>,
    JsonExtractor(user): JsonExtractor<UserUpdate>,
) -> Result<Json<User>, Error> {
    // todo need to check the email is registered already
    let user = app_state.db.user.update_user(user).await?;

    Ok(Json(user))
}

pub async fn get_user_by_id(
    State(app_state): State<AppState>,
    PathExtractor(id): PathExtractor<String>,
) -> Result<Json<User>, Error> {
    let user = app_state
        .db
        .user
        .get_user_by_id(&id)
        .await?
        .ok_or(Error::not_found())?;
    Ok(Json(user))
}

pub async fn search_user(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, pattern)): PathWithAuthExtractor<(String, String)>,
) -> Result<Json<Option<UserWithMatchType>>, Error> {
    if pattern.is_empty() || pattern.chars().count() > 32 {
        return Err(Error::bad_request("keyword is empty or too long"));
    }
    let user = app_state.db.user.search_user(&user_id, &pattern).await?;
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

    let user = app_state
        .db
        .user
        .verify_pwd(&login.account, &login.password)
        .await?
        .ok_or(Error::not_found())?;

    gen_token(&app_state, user, addr).await
}

pub async fn modify_pwd(
    State(app_state): State<AppState>,
    JsonExtractor(mut req): JsonExtractor<ModifyPwdRequest>,
) -> Result<(), Error> {
    // decode password from base64
    req.decode()?;

    // validate code
    let code = app_state
        .cache
        .get_register_code(&req.email)
        .await?
        .ok_or(Error::code_expired("code is expired"))?;
    if code != req.code {
        return Err(Error::code_invalid("code is invalid"));
    }

    app_state.db.user.modify_pwd(&req.user_id, &req.pwd).await?;

    Ok(())
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
        return Err(Error::bad_request("parameter is none".to_string()));
    }

    // get email template engine tera
    let tera = Tera::new(&state.mail_config.temp_path).map_err(Error::internal)?;
    // generate random number(validate code)
    let mut rng = rand::thread_rng();
    let num: u32 = rng.gen_range(100_000..1_000_000);

    // render email template
    let mut context = Context::new();
    context.insert("numbers", &num.to_string());
    let content = tera
        .render(&state.mail_config.temp_file, &context)
        .map_err(Error::internal)?;

    // save it to redis; expire time 5 minutes
    let msg = Message::builder()
        .from(state.mail_config.account.parse().map_err(Error::internal)?)
        .to(email.email.parse().map_err(Error::internal)?)
        .subject("Verify Login Code")
        .header(ContentType::TEXT_PLAIN)
        .multipart(
            MultiPart::alternative().singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(content),
            ),
        )
        .map_err(Error::internal)?;

    let creds = Credentials::new(state.mail_config.account, state.mail_config.password);

    // Open a remote connection to mail
    let mailer = SmtpTransport::relay(&state.mail_config.server)
        .map_err(Error::internal)?
        .credentials(creds)
        .build();

    // Send the email
    mailer.send(&msg).map_err(Error::internal)?;
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
