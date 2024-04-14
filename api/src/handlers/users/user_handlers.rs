use axum::extract::State;
use axum::Json;
use jsonwebtoken::{encode, EncodingKey, Header};
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
use abi::message::{
    CreateUserRequest, GetUserRequest, SearchUserRequest, User, UserWithMatchType, VerifyPwdRequest,
};
use utils::custom_extract::{JsonExtractor, PathExtractor, PathWithAuthExtractor};

use crate::handlers::users::{Claims, LoginRequest, Token, UserRegister};
use crate::AppState;

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
) -> Result<Json<Vec<UserWithMatchType>>, Error> {
    let mut db_rpc = app_state.db_rpc.clone();
    let request = SearchUserRequest { user_id, pattern };
    let users = db_rpc
        .search_user(request)
        .await
        .map_err(|err| Error::InternalServer(err.message().to_string()))?
        .into_inner()
        .users;
    Ok(Json(users))
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
    JsonExtractor(mut login): JsonExtractor<LoginRequest>,
) -> Result<Json<Token>, Error> {
    // decode password from base64
    login.decode()?;

    let mut db_rpc = app_state.db_rpc.clone();
    let user = db_rpc
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
    let claims = Claims::new(user.name.clone());

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(app_state.jwt_secret.as_bytes()),
    )
    .map_err(|err| Error::InternalServer(err.to_string()))?;
    app_state.cache.user_login(&user.account).await?;
    // todo get websocket address
    let ws_addr = format!(
        "ws://{}:{}/ws",
        &app_state.ws_config.host, &app_state.ws_config.port
    );

    Ok(Json(Token {
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
) -> Result<(), Error> {
    if email.email.is_empty() {
        return Err(Error::BadRequest("parameter is none".to_string()));
    }

    // get email template engine tera
    let tera = Tera::new("./api/fixtures/templates/*")
        .map_err(|e| Error::InternalServer(e.to_string()))?;
    // generate random number(validate code)
    let mut rng = rand::thread_rng();
    let num: u32 = rng.gen_range(100_000..1_000_000);

    // render email template
    let mut context = Context::new();
    context.insert("numbers", &num.to_string());
    let content = tera
        .render("email_temp.html", &context)
        .map_err(|e| Error::InternalServer(e.to_string()))?;

    // save it to redis; expire time 5 minutes
    let msg = Message::builder()
        .from(
            "653609824@qq.com"
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

    let creds = Credentials::new("653609824@qq.com".to_owned(), "rxkhmcpjgigsbegi".to_owned());

    // Open a remote connection to gmail
    let mailer = SmtpTransport::relay("smtp.qq.com")
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
