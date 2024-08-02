mod load_seq;

use clap::{command, Arg};
use tracing::{error, info};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;

use abi::config::{Component, Config};
use load_seq::load_seq;
use msg_gateway::ws_server::WsServer;

const DEFAULT_CONFIG_PATH: &str = "./config.yml";
struct LocalTimer;

impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let local_time = chrono::Local::now();
        write!(w, "{}", local_time.format("%Y-%m-%dT%H:%M:%S%.6f%:z"))
    }
}

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
            .with_timer(LocalTimer)
            .init();
    } else {
        // log to console
        tracing_subscriber::FmtSubscriber::builder()
            .with_line_number(true)
            .with_max_level(config.log.level())
            .with_timer(LocalTimer)
            .init();
    }
    // check if redis need to load seq
    load_seq(&config).await;

    match config.component {
        Component::Api => api::start(config.clone()).await,
        Component::MessageGateway => WsServer::start(config.clone()).await,
        Component::MessageServer => msg_server::start(&config).await,
        Component::All => start_all(config).await,
    }
}

async fn start_all(config: Config) {
    // start ws rpc server
    let cloned_config = config.clone();
    let ws_server = tokio::spawn(async move {
        WsServer::start(cloned_config).await;
    });

    // start cleaner
    db::clean_receive_box(&config).await;

    // start api server
    let cloned_config = config.clone();
    let api_server = tokio::spawn(async move {
        api::start(cloned_config).await;
    });

    // start consumer rpc server
    let consumer_server = tokio::spawn(async move {
        msg_server::start(&config).await;
    });

    // wait all server stop
    tokio::select! {
        _ = ws_server => {error!("ws server down!")},
        _ = api_server => {error!("api server down!")},
        _ = consumer_server => {error!("consumer server down!")},
    }
}
