use abi::config::Config;
use chat::ChatRpcService;
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    let config = Config::load("../abi/fixtures/im.yml").unwrap();
    ChatRpcService::start(&config).await;
}
