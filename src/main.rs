mod domain;
mod errors;
mod handlers;
mod infra;
mod routes;
mod service;
mod utils;

use crate::errors::internal_error;
use crate::routes::app_routes;
use deadpool_diesel::postgres::{Manager, Pool};
use domain::model::manager;
use dotenvy::dotenv;
use std::env;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tracing::Level;
use crate::handlers::users::UserRequest;

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
    dotenv().expect("need .env file");
    // create pool
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    // Create a connection pool to the PostgreSQL database
    let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
    let pool = Pool::builder(manager).build().unwrap();
    let app_state = AppState { pool, hub };
    let app = app_routes(app_state.clone()).with_state(app_state);

    // let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let address = "0.0.0.0:3000".to_string();

    let socket_addr: SocketAddr = address.parse().unwrap();
    tracing::debug!("listening on {}", address);
    axum::Server::bind(&socket_addr)
        .serve(app.into_make_service())
        .await
        .map_err(internal_error)
        .unwrap()
}
