use abi::config::Config;
use ws::WsServer;

#[tokio::main]
async fn main() {
    WsServer::start(Config::load("./abi/fixtures/im.yml").unwrap()).await
}
