use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use hyper::{header, StatusCode};
use oauth2::{
    basic::{BasicClient, BasicTokenType},
    reqwest::async_http_client,
    AuthorizationCode, CsrfToken, EmptyExtraTokenFields, Scope, StandardTokenResponse,
    TokenResponse,
};
use serde::Deserialize;
use tracing::info;

use crate::AppState;

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
    State(state): State<AppState>,
) -> impl IntoResponse {
    let token_result = get_token_result(&state.oauth2_clients.github, auth.code).await;

    let access_token = token_result.access_token().secret();
    let user_info = reqwest::Client::new()
        .get(&state.oauth2_config.github.user_info_url)
        .header(
            header::AUTHORIZATION,
            format!("{} {}", AUTHORIZATION_HEADER_PREFIX, access_token),
        )
        .header(header::USER_AGENT, USER_AGENT)
        .send()
        .await;

    // 获取用户的 email
    let user_emails = reqwest::Client::new()
        .get(&state.oauth2_config.github.email_url)
        .header(
            header::AUTHORIZATION,
            format!("{} {}", AUTHORIZATION_HEADER_PREFIX, access_token),
        )
        .header(header::USER_AGENT, USER_AGENT)
        .send()
        .await;

    match (user_info, user_emails) {
        (Ok(user_resp), Ok(emails_resp)) => {
            let user: GitHubUser = user_resp.json().await.unwrap_or_default();
            let emails: Vec<GitHubEmail> = emails_resp.json().await.unwrap_or_default();

            (
                StatusCode::OK,
                format!(
                    "GitHub login successful. User info: {:?}, Emails: {:?}",
                    user, emails
                ),
            )
        }
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to get user info or emails".into(),
        ),
    }
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
) -> StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType> {
    client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(async_http_client)
        .await
        .unwrap()
}
