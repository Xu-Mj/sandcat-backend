use abi::config::Config;
use tracing::Level;
use ws::ws_server::WsServer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    WsServer::start(Config::load("./abi/fixtures/im.yml").unwrap()).await
}
