use tracing::Level;

use abi::config::Config;
use msg_gateway::ws_server::WsServer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    WsServer::start(Config::load("config.yml").unwrap()).await
}
#[cfg(test)]
mod tests {
    use abi::message::msg_service_server::MsgServiceServer;
    use abi::message::Msg;
    use msg_gateway::rpc;
    use tonic::server::NamedService;

    #[test]
    fn test_load() {
        let msg = Msg::default();
        println!("{}", serde_json::to_string(&msg).unwrap());
        println!(
            "{:?}",
            <MsgServiceServer<rpc::MsgRpcService> as NamedService>::NAME
        );
    }
}
