use tracing::Level;

use abi::config::Config;
use db::rpc::DbRpcService;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    let config = Config::load("./abi/fixtures/im.yml").unwrap();
    DbRpcService::start(&config).await;
}
