mod config;
mod domain;
mod errors;
mod handlers;
mod infra;
mod routes;
mod utils;

use crate::routes::app_routes;
use abi::config::Config;
use abi::message::chat_service_server::ChatServiceServer;
use chat::ChatRpcService;
use deadpool_diesel::postgres::{Manager, Pool};
use domain::model::manager;
use kafka::producer::{Producer, RequiredAcks};
use redis::Client;
use sqlx::PgPool;
use std::time::Duration;
use tokio::sync::mpsc;
use tonic::transport::Server;
use tracing::Level;

#[derive(Clone)]
pub struct AppState {
    pool: Pool,
    pg_pool: PgPool,
    hub: manager::Manager,
    redis: Client,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    let config = Config::load("./abi/fixtures/im.yml").unwrap();
    let producer = Producer::from_hosts(config.kafka.hosts)
        // ~ give the brokers one second time to ack the message
        .with_ack_timeout(Duration::from_secs(1))
        // ~ require only one broker to ack the message
        .with_required_acks(RequiredAcks::One)
        // ~ build the producer with the above settings
        .create()
        .expect("Producer creation error");
    let service = ChatRpcService::new(producer);
    let service = ChatServiceServer::new(service);
    let rpc_addr = format!("{}:{}", config.rpc.chat.host, config.rpc.chat.port);
    tokio::spawn(async move {
        Server::builder()
            .add_service(service)
            .serve(rpc_addr.parse().unwrap())
            .await
            .unwrap();
    });
    let (tx, rx) = mpsc::channel(1024);
    // create pool
    let database_url = config::config().await.db_url();
    // Create a connection pool to the PostgreSQL database
    let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
    let pool = Pool::builder(manager).build().unwrap();
    let pg_pool = PgPool::connect(database_url).await.unwrap();
    let redis = Client::open(config::config().await.redis_url()).expect("redis can't open");
    let hub = manager::Manager::new(tx);
    let mut cloned_hub = hub.clone();
    let cloned_pool = pool.clone();
    let cloned_pg_pool = pg_pool.clone();
    let cloned_redis = redis.clone();
    tokio::spawn(async move {
        cloned_hub
            .run(rx, cloned_pool, cloned_pg_pool, cloned_redis)
            .await;
    });
    let app_state = AppState {
        pool,
        pg_pool,
        hub,
        redis,
    };
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
