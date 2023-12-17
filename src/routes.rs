use crate::handlers::users::{create_user, get_user_by_id};
use crate::service::ws::websocket_handler;
use crate::AppState;
use axum::routing::{get, post};
use axum::Router;

pub(crate) fn app_routes(state: AppState) -> Router {
    Router::new()
        .nest("/user", user_routes(state.clone()))
        .nest("/ws", ws_routes(state.clone()))
}

fn user_routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_user))
        .route("/:id", get(get_user_by_id))
        .with_state(state)
}

fn ws_routes(state: AppState) -> Router {
    Router::new()
        .route("/:id", get(websocket_handler))
        .with_state(state)
}
