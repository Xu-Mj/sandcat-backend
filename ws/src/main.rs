use abi::config::Config;
use ws::ws_server::WsServer;

#[tokio::main]
async fn main() {
    WsServer::start(Config::load("./abi/fixtures/im.yml").unwrap()).await
}
