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
#[cfg(test)]
mod tests {
    use abi::message::Msg;
    #[test]
    fn test_load() {
        let msg = Msg::default();
        println!("{}", serde_json::to_string(&msg).unwrap());
    }
}
