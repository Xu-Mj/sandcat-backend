use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::handlers::files::file::{get_file_by_name, upload};
use crate::handlers::friends::friend_handlers::{
    agree, black_list, create, deny, get_apply_list_by_user_id, get_friends_list_by_user_id2,
    update_friend_remark,
};
use crate::handlers::groups::create_group_handler;
use crate::handlers::users::user_handlers::{logout, search_user};
use crate::handlers::users::{create_user, get_user_by_id, login, send_email};
use crate::handlers::ws::websocket_handler;
use crate::AppState;

pub(crate) fn app_routes(state: AppState) -> Router {
    Router::new()
        .nest("/user", user_routes(state.clone()))
        .nest("/friend", friend_routes(state.clone()))
        .nest("/ws", ws_routes(state.clone()))
        .nest("/file", file_routes(state.clone()))
        .nest("/group", group_routes(state.clone()))
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

fn friend_routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create))
        .route("/:id", get(get_friends_list_by_user_id2))
        .route("/:id/apply", get(get_apply_list_by_user_id))
        .route("/agree", put(agree))
        .route("/blacklist", put(black_list))
        .route("/remark", put(update_friend_remark))
        .route("/:user_id/deny/:friend_id", post(deny))
        .with_state(state)
}

fn ws_routes(state: AppState) -> Router {
    Router::new()
        .route("/:user_id/conn/:token/:pointer_id", get(websocket_handler))
        .with_state(state)
}

fn file_routes(state: AppState) -> Router {
    Router::new()
        .route("/upload", post(upload))
        .route("/get/:filename", get(get_file_by_name))
        .with_state(state)
}

fn group_routes(state: AppState) -> Router {
    Router::new()
        .route("/:user_id", post(create_group_handler))
        .with_state(state)
}
