use abi::config::Config;
use consumer::ConsumerService;
use tracing::{info, Level};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let config = Config::load("../abi/fixtures/im.yml").unwrap();

    let mut consumer = ConsumerService::new(&config).await;
    info!("start consumer...");
    info!(
        "connect to kafka server: {:?}; topic: {}, group: {}",
        config.kafka.hosts, config.kafka.topic, config.kafka.group
    );
    info!("connect to rpc server: {}", config.rpc.db.rpc_server_url());
    if let Err(e) = consumer.consume().await {
        panic!("failed to consume message, error: {}", e);
    }
}
