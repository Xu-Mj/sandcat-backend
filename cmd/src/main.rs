use abi::config::{Component, Config};
use chat::ChatRpcService;
use clap::{command, Arg};
use consumer::ConsumerService;
use db::rpc::DbRpcService;
use pusher::PusherRpcService;
use tracing::{error, info};
use ws::ws_server::WsServer;

const DEFAULT_CONFIG_PATH: &str = "./config.yml";

#[tokio::main]
async fn main() {
    // chat rely on kafka server
    // consumer rely on kafka server;

    // ws rely on chat;
    // consumer rely on db and pusher rpc server;

    // get configuration path
    let matches = command!()
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("configuration")
                .short('p')
                .long("configuration")
                .value_name("CONFIGURATION")
                .default_value(DEFAULT_CONFIG_PATH)
                .help("Set the configuration path"),
        )
        .get_matches();
    let default_config = DEFAULT_CONFIG_PATH.to_string();
    let configuration = matches
        .get_one::<String>("configuration")
        .unwrap_or(&default_config);

    info!("load configuration from: {:?}", configuration);
    let config = Config::load(configuration).unwrap();

    // init tracing
    if config.log.output != "console" {
        // redirect log to file
        let file_appender = tracing_appender::rolling::daily(&config.log.output, "sandcat");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        // builder = builder.with_writer(non_blocking);
        tracing_subscriber::FmtSubscriber::builder()
            .with_line_number(true)
            .with_max_level(config.log.level())
            .with_writer(non_blocking)
            .init();
    } else {
        // log to console
        tracing_subscriber::FmtSubscriber::builder()
            .with_line_number(true)
            .with_max_level(config.log.level())
            .init();
    }

    match config.component {
        Component::Chat => ChatRpcService::start(&config).await,
        Component::Consumer => ConsumerService::new(&config).await.consume().await.unwrap(),
        Component::Db => DbRpcService::start(&config).await,
        Component::Pusher => api::start(config.clone()).await,
        Component::Ws => WsServer::start(config.clone()).await,
        Component::All => start_all(config).await,
    }
}

async fn start_all(config: Config) {
    // start chat rpc server
    let cloned_config = config.clone();
    let chat_server = tokio::spawn(async move {
        ChatRpcService::start(&cloned_config).await;
    });

    // start ws rpc server
    let cloned_config = config.clone();
    let ws_server = tokio::spawn(async move {
        WsServer::start(cloned_config).await;
    });

    // start db rpc server
    let cloned_config = config.clone();
    let db_server = tokio::spawn(async move {
        DbRpcService::start(&cloned_config).await;
    });

    // start pusher rpc server
    let cloned_config = config.clone();
    let pusher_server = tokio::spawn(async move {
        PusherRpcService::start(&cloned_config).await;
    });

    // start api server
    let cloned_config = config.clone();
    let api_server = tokio::spawn(async move {
        api::start(cloned_config).await;
    });

    // start consumer rpc server
    let consumer_server = tokio::spawn(async move {
        if let Err(e) = ConsumerService::new(&config).await.consume().await {
            panic!("failed to consume message, error: {:?}", e);
        }
    });

    // wait all server stop
    tokio::select! {
        _ = chat_server => {error!("chat server down!")},
        _ = ws_server => {error!("ws server down!")},
        _ = db_server => {error!("database server down!")},
        _ = pusher_server => {error!("pusher server down!")},
        _ = api_server => {error!("api server down!")},
        _ = consumer_server => {error!("consumer server down!")},
    }
}
