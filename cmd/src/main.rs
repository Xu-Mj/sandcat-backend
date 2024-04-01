use abi::config::Config;
use chat::ChatRpcService;
use consumer::ConsumerService;
use db::rpc::DbRpcService;
use pusher::PusherRpcService;
use tracing::{info, Level};
use ws::ws_server::WsServer;

#[tokio::main]
async fn main() {
    // chat rely on kafka server
    // consumer rely on kafka server;

    // ws rely on chat;
    // consumer rely on db and pusher rpc server;

    // init tracing
    tracing_subscriber::FmtSubscriber::builder()
        .with_line_number(true)
        .with_max_level(Level::DEBUG)
        .init();

    let config = Config::load("./abi/fixtures/im.yml").unwrap();

    // start chat rpc server
    let cloned_config = config.clone();
    let chat_server = tokio::spawn(async move {
        ChatRpcService::start(&cloned_config).await.unwrap();
    });

    // start ws rpc server
    let cloned_config = config.clone();
    let ws_server = tokio::spawn(async move {
        WsServer::start(cloned_config).await;
    });

    // start db rpc server
    let cloned_config = config.clone();
    let db_server = tokio::spawn(async move {
        DbRpcService::start(&cloned_config).await.unwrap();
    });

    // start pusher rpc server
    let cloned_config = config.clone();
    let pusher_server = tokio::spawn(async move {
        PusherRpcService::start(&cloned_config).await.unwrap();
    });

    // start consumer rpc server
    let consumer_server = tokio::spawn(async move {
        let mut consumer = ConsumerService::new(&config).await;
        info!("start consumer...");
        info!(
            "connect to kafka server: {:?}; topic: {}, group: {}",
            config.kafka.hosts, config.kafka.topic, config.kafka.group
        );
        info!("connect to rpc server: {}", config.rpc.db.rpc_server_url());
        if let Err(e) = consumer.consume().await {
            panic!("failed to consume message, error: {}", e);
        }
    });

    // wait all server stop
    tokio::select! {
        _ = chat_server => {},
        _ = ws_server => {},
        _ = db_server => {},
        _ = pusher_server => {},
        _ = consumer_server => {},
    }
}
