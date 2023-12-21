use crate::handlers::users::user_handlers::logout;
use crate::handlers::users::{create_user, get_user_by_id, login};
use crate::service::ws::ws_service::websocket_handler;
use crate::AppState;
use axum::routing::{delete, get, post};
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
        .route("/login", post(login))
        .route("/logout/:uuid", delete(logout))
        .with_state(state)
}

fn ws_routes(state: AppState) -> Router {
    Router::new()
        .route("/:id", get(websocket_handler))
        .with_state(state)
}
