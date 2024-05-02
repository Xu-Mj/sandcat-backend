use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, post, put};
use axum::{Extension, Router};

use crate::handlers::files::file::{get_file_by_name, upload};
use crate::handlers::friends::friend_handlers::{
    agree, create_friendship, delete_friend, get_apply_list_by_user_id,
    get_friends_list_by_user_id, update_friend_remark,
};
use crate::handlers::groups::group_handlers::{
    create_group_handler, delete_group_handler, invite_new_members, update_group_handler,
};
use crate::handlers::messages::msg_handlers::{del_msg, get_seq, pull_offline_messages};
use crate::handlers::users::{
    create_user, get_user_by_id, login, logout, refresh_token, search_user, send_email, update_user,
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
        .layer(Extension(state.clone()))
        .route("/:id", get(get_friends_list_by_user_id))
        .layer(Extension(state.clone()))
        .route("/:id/apply", get(get_apply_list_by_user_id))
        .layer(Extension(state.clone()))
        .route("/agree", put(agree))
        .layer(Extension(state.clone()))
        .route("/", delete(delete_friend))
        .layer(Extension(state.clone()))
        .route("/remark", put(update_friend_remark))
        .layer(Extension(state.clone()))
        // .route("/:user_id/deny/:friend_id", post(deny))
        .with_state(state)
}

fn user_routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_user))
        .layer(Extension(state.clone()))
        .route("/", put(update_user))
        .layer(Extension(state.clone()))
        .route("/:id", get(get_user_by_id))
        .layer(Extension(state.clone()))
        .route("/refresh_token/:token", get(refresh_token))
        .layer(Extension(state.clone()))
        .route("/:user_id/search/:pattern", get(search_user))
        .layer(Extension(state.clone()))
        .route("/login", post(login))
        .route("/logout/:uuid", delete(logout))
        .layer(Extension(state.clone()))
        .route("/mail/send", post(send_email))
        .layer(Extension(state.clone()))
        .with_state(state)
}

fn group_routes(state: AppState) -> Router {
    Router::new()
        .route("/:user_id", post(create_group_handler))
        .layer(Extension(state.clone()))
        .route("/invite", put(invite_new_members))
        .layer(Extension(state.clone()))
        .route("/", delete(delete_group_handler))
        .layer(Extension(state.clone()))
        .route("/update/:user_id", put(update_group_handler))
        .layer(Extension(state.clone()))
        .with_state(state)
}

const MAX_FILE_UPLOAD_SIZE: usize = 1024 * 1024 * 50;
fn file_routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/upload",
            post(upload).layer(DefaultBodyLimit::max(MAX_FILE_UPLOAD_SIZE)),
        )
        .layer(Extension(state.clone()))
        .route("/get/:filename", get(get_file_by_name))
        .layer(Extension(state.clone()))
        .with_state(state)
}

fn msg_routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(pull_offline_messages))
        .layer(Extension(state.clone()))
        .route("/seq/:user_id", get(get_seq))
        .layer(Extension(state.clone()))
        .route("/", delete(del_msg))
        .with_state(state)
}
