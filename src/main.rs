mod config;
mod domain;
mod errors;
mod handlers;
mod infra;
mod routes;
mod service;
mod utils;

use crate::routes::app_routes;
use deadpool_diesel::postgres::{Manager, Pool};
use domain::model::manager;
use dotenvy::dotenv;
use std::env;
use tokio::sync::mpsc;
use tracing::Level;
use tracing_subscriber::fmt::format;

#[derive(Clone)]
pub struct AppState {
    pool: Pool,
    hub: manager::Manager,
}
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    let (tx, rx) = mpsc::channel(1024);
    let hub = manager::Manager::new(tx);
    let mut cloned_hub = hub.clone();
    tokio::spawn(async move {
        cloned_hub.run(rx).await;
    });
    // create pool
    let database_url = config::config().await.db_url();
    // Create a connection pool to the PostgreSQL database
    let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
    let pool = Pool::builder(manager).build().unwrap();
    let app_state = AppState { pool, hub };
    let app = app_routes(app_state);

    let addr = format!(
        "{}:{}",
        config::config().await.server_host(),
        config::config().await.server_port()
    );
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
