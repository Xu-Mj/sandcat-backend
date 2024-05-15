use abi::config::Config;
use pusher::PusherRpcService;
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_line_number(true)
        .init();
    let config = Config::load("config.yml").unwrap();
    PusherRpcService::start(&config).await;
}
