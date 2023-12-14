mod service;
mod model;

use axum::{
    routing::get,
    Router,
};
use tokio::sync::mpsc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use crate::model::manager::Manager;
use crate::service::ws::websocket_handler;




#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_chat=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
