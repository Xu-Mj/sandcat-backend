use std::sync::Arc;

use tonic::transport::Channel;

use abi::config::{Config, MailConfig, WsServerConfig};
use abi::errors::Error;
use abi::message::db_service_client::DbServiceClient;
use abi::message::msg_service_client::MsgServiceClient;
use cache::Cache;
use oss::Oss;

mod api_utils;
pub(crate) mod handlers;
pub(crate) mod routes;

#[derive(Clone, Debug)]
pub struct AppState {
    pub db_rpc: DbServiceClient<Channel>,
    pub ws_rpc: MsgServiceClient<Channel>,
    pub cache: Arc<Box<dyn Cache>>,
    pub oss: Arc<Box<dyn Oss>>,
    pub ws_config: WsServerConfig,
    pub mail_config: MailConfig,
    pub jwt_secret: String,
}

impl AppState {
    pub async fn new(config: &Config) -> Self {
        let ws_rpc = Self::get_ws_rpc_client(config).await.unwrap();

        let db_rpc = Self::get_db_rpc_client(config).await.unwrap();

        let cache = Arc::new(cache::cache(config));

        let oss = Arc::new(oss::oss(config).await);

        let ws_config = config.websocket.clone();

        let mail_config = config.mail.clone();

        Self {
            ws_rpc,
            db_rpc,
            cache,
            oss,
            ws_config,
            mail_config,
            jwt_secret: config.server.jwt_secret.clone(),
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
        let channel =
            utils::get_rpc_channel_by_name(config, &config.rpc.ws.name, &config.rpc.ws.protocol)
                .await?;
        let ws_rpc = MsgServiceClient::new(channel);
        Ok(ws_rpc)
    }
}

pub async fn start(config: Config) {
    let state = AppState::new(&config).await;
    let app = routes::app_routes(state.clone());
    let listener = tokio::net::TcpListener::bind(&config.server.server_url())
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
