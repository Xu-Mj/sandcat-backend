use tracing::Level;

use abi::config::Config;
use db::rpc::DbRpcService;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_line_number(true)
        .init();

    let config = Config::load("config.yml").unwrap();

    // start cleaner
    db::clean_receive_box(&config).await;

    // start rpc service
    DbRpcService::start(&config).await;
}
