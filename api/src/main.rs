use std::sync::Arc;

use abi::config::Config;
use abi::errors::Error;
use tonic::transport::Channel;
use tracing::Level;

use abi::message::db_service_client::DbServiceClient;
use abi::message::msg_service_client::MsgServiceClient;
use cache::Cache;

mod handlers;
mod routes;

#[derive(Clone, Debug)]
pub struct AppState {
    pub db_rpc: DbServiceClient<Channel>,
    pub ws_rpc: MsgServiceClient<Channel>,
    pub cache: Arc<Box<dyn Cache>>,
}

impl AppState {
    pub async fn new(config: &Config) -> Self {
        let ws_rpc = Self::get_ws_rpc_client(config).await.unwrap();

        let db_rpc = Self::get_db_rpc_client(config).await.unwrap();

        let cache = Arc::new(cache::cache(config));

        Self {
            ws_rpc,
            db_rpc,
            cache,
        }
    }

    async fn get_db_rpc_client(config: &Config) -> Result<DbServiceClient<Channel>, Error> {
        // use service register center to get ws rpc url
        let channel =
            utils::get_rpc_channel_by_name(config, &config.rpc.db.name, &config.rpc.db.protocol)
                .await?;
        let db_rpc = DbServiceClient::new(channel);
        Ok(db_rpc)
    }

    async fn get_ws_rpc_client(config: &Config) -> Result<MsgServiceClient<Channel>, Error> {
        let channel = utils::get_rpc_channel_by_name(
            config,
            &config.rpc.pusher.name,
            &config.rpc.pusher.protocol,
        )
        .await?;
        let push_rpc = MsgServiceClient::new(channel);
        Ok(push_rpc)
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let config = Config::load("../abi/fixtures/im.yml").unwrap();
    let state = AppState::new(&config).await;
    let app = routes::app_routes(state);
    let listener = tokio::net::TcpListener::bind(&config.server.url(false))
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
