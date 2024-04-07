use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::handlers::files::file::{get_file_by_name, upload};
use crate::handlers::friends::friend_handlers::{
    agree, create_friendship, get_apply_list_by_user_id, get_friends_list_by_user_id,
    update_friend_remark,
};
use crate::handlers::groups::group_handlers::{
    create_group_handler, delete_group_handler, invite_new_members, update_group_handler,
};
use crate::handlers::messages::msg_handlers::pull_offline_messages;
use crate::handlers::users::{create_user, get_user_by_id, login, logout, search_user, send_email};
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
        .route("/:id", get(get_friends_list_by_user_id))
        .route("/:id/apply", get(get_apply_list_by_user_id))
        .route("/agree", put(agree))
        // .route("/blacklist", put(black_list))
        .route("/remark", put(update_friend_remark))
        // .route("/:user_id/deny/:friend_id", post(deny))
        .with_state(state)
}

fn user_routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_user))
        .route("/:id", get(get_user_by_id))
        .route("/:user_id/search/:pattern", get(search_user))
        .route("/login", post(login))
        .route("/logout/:uuid", delete(logout))
        .route("/mail/send", post(send_email))
        .with_state(state)
}

fn group_routes(state: AppState) -> Router {
    Router::new()
        .route("/:user_id", post(create_group_handler))
        .route("/invite", put(invite_new_members))
        .route("/", delete(delete_group_handler))
        .route("/update/:user_id", put(update_group_handler))
        .with_state(state)
}

fn file_routes(state: AppState) -> Router {
    Router::new()
        .route("/upload", post(upload))
        .route("/get/:filename", get(get_file_by_name))
        .with_state(state)
}

fn msg_routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(pull_offline_messages))
        .with_state(state)
}
