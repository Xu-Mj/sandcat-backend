use abi::config::Config;
use pusher::PusherRpcService;

#[tokio::main]
async fn main() {
    let config = Config::load("./abi/fixtures/im.yml").unwrap();
    PusherRpcService::start(&config).await;
}
