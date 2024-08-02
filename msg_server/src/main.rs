use tracing::Level;

use abi::config::Config;

use msg_server::productor::ChatRpcService;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    let config = Config::load("config.yml").unwrap();
    ChatRpcService::start(&config).await;
}
