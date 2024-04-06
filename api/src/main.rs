use abi::config::Config;
use tracing::Level;

use api::start;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_max_level(Level::DEBUG)
        .init();

    let config = Config::load("./abi/fixtures/im.yml").unwrap();
    start(config).await;
}
