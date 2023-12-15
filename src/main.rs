mod model;
mod service;

use crate::model::manager::Manager;
use crate::service::ws::websocket_handler;
use axum::{routing::get, Router};
use tokio::sync::mpsc;
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    let (tx, rx) = mpsc::channel(1024);
    let hub = Manager::new(tx);
    let mut cloned_hub = hub.clone();
    tokio::spawn(async move {
        cloned_hub.run(rx).await;
    });

    let app = Router::new()
        .route("/ws/:id", get(websocket_handler))
        .with_state(hub);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
