use std::net::SocketAddr;

use abi::{
    errors::Error,
    message::{CreateUserRequest, GetUserByEmailRequest, User},
};
use axum::{
    extract::{ConnectInfo, Query, State},
    response::{IntoResponse, Redirect},
    Json,
};
use hyper::header;
use nanoid::nanoid;
use oauth2::{
    basic::{BasicClient, BasicTokenType},
    reqwest::async_http_client,
    AuthorizationCode, CsrfToken, EmptyExtraTokenFields, Scope, StandardTokenResponse,
    TokenResponse,
};
use serde::Deserialize;
use tracing::info;

use crate::AppState;

use super::{gen_token, Token};

const AUTHORIZATION_HEADER_PREFIX: &str = "Bearer ";
const USER_AGENT: &str = "SandCat-Auth";

pub async fn github_login(State(state): State<AppState>) -> impl IntoResponse {
    let authorize_url = get_auth_url(&state.oauth2_clients.github).await;

    Redirect::temporary(&authorize_url.to_string())
}

/// todo need to handle state, use cache to store state, and verify it in callback
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AuthResp {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct GitHubUser {
    pub name: String,
    pub avatar_url: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct GitHubEmail {
    pub email: String,
    pub primary: bool,
}

pub async fn github_callback(
    Query(auth): Query<AuthResp>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> Result<Json<Token>, Error> {
    let token_result = get_token_result(&state.oauth2_clients.github, auth.code).await?;

    let access_token = token_result.access_token().secret();

    // request user's email
    let user_emails: Vec<GitHubEmail> = reqwest::Client::new()
        .get(&state.oauth2_config.github.email_url)
        .header(
            header::AUTHORIZATION,
            format!("{} {}", AUTHORIZATION_HEADER_PREFIX, access_token),
        )
        .header(header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .json()
        .await?;

    let email = user_emails
        .into_iter()
        .find(|item| item.primary)
        .ok_or(Error::not_found())?;

    // select user info from db by email
    let mut db_rpc = state.db_rpc.clone();
    let req = GetUserByEmailRequest {
        email: email.email.clone(),
    };
    let mut user_info = db_rpc.get_user_by_email(req).await?.into_inner().user;

    // if none, need to get user info from github and register
    if user_info.is_none() {
        user_info = Some(register_user(&state, email.email, access_token).await?);
    }

    gen_token(&state, user_info.unwrap(), addr).await
}

pub async fn google_login(State(state): State<AppState>) -> impl IntoResponse {
    let authorize_url = get_auth_url(&state.oauth2_clients.google).await;

    Redirect::temporary(&authorize_url.to_string())
}

pub async fn google_callback(
    Query(auth): Query<AuthResp>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let token_result = get_token_result(&state.oauth2_clients.google, auth.code).await;
    info!("{:?}", token_result);
}

async fn get_auth_url(client: &BasicClient) -> String {
    let (authorize_url, _csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(String::from("read:user")))
        .add_scope(Scope::new(String::from("user:email")))
        .url();
    authorize_url.to_string()
}

async fn get_token_result(
    client: &BasicClient,
    code: String,
) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, Error> {
    client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(async_http_client)
        .await
        .map_err(Error::internal)
}

async fn download_avatar(url: &str, state: &AppState) -> Result<String, Error> {
    let content = reqwest::get(url).await?.bytes().await?;

    // get image type
    let mut filename = nanoid!();
    if let Ok(tp) = image::guess_format(&content) {
        filename = format!("{}.{}", filename, tp.extensions_str().first().unwrap());
    }

    let oss = state.oss.clone();
    oss.upload_avatar(&filename, content.into()).await?;
    Ok(filename)
}

async fn register_user(state: &AppState, email: String, access_token: &str) -> Result<User, Error> {
    let user: GitHubUser = reqwest::Client::new()
        .get(&state.oauth2_config.github.user_info_url)
        .header(
            header::AUTHORIZATION,
            format!("{} {}", AUTHORIZATION_HEADER_PREFIX, access_token),
        )
        .header(header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .json()
        .await?;

    // download avatar from github

    let avatar = download_avatar(&user.avatar_url, state).await?;

    let id = nanoid!();
    // convert user to db user
    let user2db = User {
        id: id.clone(),
        name: user.name,
        account: id,
        email: Some(email),
        avatar,
        ..Default::default()
    };

    let mut db_rpc = state.db_rpc.clone();
    let request = CreateUserRequest {
        user: Some(user2db),
    };
    let response = db_rpc.create_user(request).await?;

    response
        .into_inner()
        .user
        .ok_or(Error::internal_with_details(
            "create user failed, user is none",
        ))
}
