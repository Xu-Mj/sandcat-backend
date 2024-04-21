use abi::config::Config;
use chat::ChatRpcService;
use clap::{command, Arg};
use consumer::ConsumerService;
use db::rpc::DbRpcService;
use pusher::PusherRpcService;
use tracing::info;
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

    // start api server
    let cloned_config = config.clone();
    let api_server = tokio::spawn(async move {
        api::start(cloned_config).await;
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
        _ = api_server => {},
        _ = consumer_server => {},
    }
}
