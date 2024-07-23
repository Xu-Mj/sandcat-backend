use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::handlers::files::file::{get_avatar_by_name, get_file_by_name, upload, upload_avatar};
use crate::handlers::friends::friend_handlers::{
    agree, create_friendship, delete_friend, get_apply_list_by_user_id,
    get_friends_list_by_user_id, query_friend_info, update_friend_remark,
};
use crate::handlers::groups::group_handlers::{
    create_group_handler, delete_group_handler, get_group, get_group_and_members,
    get_group_members, invite_new_members, remove_member, update_group_handler,
};
use crate::handlers::messages::msg_handlers::{del_msg, get_seq, pull_offline_messages};
use crate::handlers::users::{
    create_user, get_user_by_id, github_callback, github_login, google_callback, google_login,
    login, logout, refresh_token, search_user, send_email, update_user,
};
use crate::AppState;

pub(crate) fn app_routes(state: AppState) -> Router {
    Router::new()
        .nest("/user", user_routes(state.clone()))
        .nest("/friend", friend_routes(state.clone()))
        .nest("/file", file_routes(state.clone()))
        .nest("/group", group_routes(state.clone()))
        .nest("/message", msg_routes(state.clone()))
}

fn friend_routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_friendship))
        .route("/:id/:offline_time", get(get_friends_list_by_user_id))
        .route("/:id/apply", get(get_apply_list_by_user_id))
        .route("/agree", put(agree))
        .route("/", delete(delete_friend))
        .route("/remark", put(update_friend_remark))
        .route("/query/:user_id", get(query_friend_info))
        .with_state(state)
}

fn user_routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_user))
        .route("/", put(update_user))
        .route("/:id", get(get_user_by_id))
        .route("/refresh_token/:token/:is_refresh", get(refresh_token))
        .route("/:user_id/search/:pattern", get(search_user))
        .route("/login", post(login))
        .route("/logout/:uuid", delete(logout))
        .route("/mail/send", post(send_email))
        .route("/auth/wechat", get(google_login))
        .route("/auth/wechat/callback", get(google_callback))
        .route("/auth/github", get(github_login))
        .route("/auth/github/callback", get(github_callback))
        .with_state(state)
}

fn group_routes(state: AppState) -> Router {
    Router::new()
        .route("/:user_id/:group_id", get(get_group))
        .route("/:user_id", post(create_group_handler))
        .route("/invite", put(invite_new_members))
        .route("/", delete(delete_group_handler))
        .route("/:user_id", put(update_group_handler))
        .route("/member/:user_id/:group_id", get(get_group_and_members))
        .route("/member", post(get_group_members))
        .route("/member", delete(remove_member))
        .with_state(state)
}

const MAX_FILE_UPLOAD_SIZE: usize = 1024 * 1024 * 50;
fn file_routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/upload",
            post(upload).layer(DefaultBodyLimit::max(MAX_FILE_UPLOAD_SIZE)),
        )
        .route("/get/:filename", get(get_file_by_name))
        .route(
            "/avatar/upload",
            post(upload_avatar).layer(DefaultBodyLimit::max(MAX_FILE_UPLOAD_SIZE)),
        )
        .route("/avatar/get/:filename", get(get_avatar_by_name))
        .with_state(state)
}

fn msg_routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(pull_offline_messages))
        .route("/seq/:user_id", get(get_seq))
        .route("/", delete(del_msg))
        .with_state(state)
}
