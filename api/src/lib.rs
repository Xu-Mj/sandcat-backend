use std::net::SocketAddr;
use std::sync::Arc;

use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use synapse::service::client::ServiceClient;
use xdb::searcher_init;

use abi::config::{Config, MailConfig, OAuth2, OAuth2Item, WsServerConfig};
use abi::message::chat_service_client::ChatServiceClient;
use abi::message::db_service_client::DbServiceClient;
use cache::Cache;
use oss::Oss;
use utils::service_discovery::LbWithServiceDiscovery;

use crate::api_utils::lb;

mod api_utils;
pub(crate) mod handlers;
pub(crate) mod routes;

#[derive(Clone, Debug)]
pub struct AppState {
    pub db_rpc: DbServiceClient<LbWithServiceDiscovery>,
    pub chat_rpc: ChatServiceClient<LbWithServiceDiscovery>,
    pub cache: Arc<dyn Cache>,
    pub oss: Arc<dyn Oss>,
    pub ws_lb: Arc<lb::LoadBalancer>,
    pub ws_config: WsServerConfig,
    pub mail_config: MailConfig,
    pub jwt_secret: String,
    pub oauth2_config: OAuth2,
    pub oauth2_clients: OAuth2Clients,
}

#[derive(Clone, Debug)]
pub struct OAuth2Clients {
    pub github: BasicClient,
    pub google: BasicClient,
}

impl AppState {
    pub async fn new(config: &Config) -> Self {
        let db_rpc = utils::get_rpc_client(config, config.rpc.db.name.clone())
            .await
            .unwrap();
        let chat_rpc = utils::get_rpc_client(config, config.rpc.chat.name.clone())
            .await
            .unwrap();

        let cache = cache::cache(config);

        let oss = oss::oss(config).await;

        let ws_config = config.websocket.clone();

        let mail_config = config.mail.clone();
        let client = ServiceClient::builder()
            .server_host(config.service_center.host.clone())
            .server_port(config.service_center.port)
            .build()
            .await
            .expect("build service client failed");
        let ws_lb = Arc::new(
            lb::LoadBalancer::new(
                config.websocket.name.clone(),
                config.server.ws_lb_strategy.clone(),
                client,
            )
            .await,
        );

        let oauth2_config = config.server.oauth2.clone();
        let oauth2_clients = init_oauth2(config);

        Self {
            db_rpc,
            cache,
            oss,
            ws_lb,
            ws_config,
            mail_config,
            jwt_secret: config.server.jwt_secret.clone(),
            chat_rpc,
            oauth2_config,
            oauth2_clients,
        }
    }
}

fn init_oauth2(config: &Config) -> OAuth2Clients {
    let google = init_oauth2_client(&config.server.oauth2.wechat);
    let github = init_oauth2_client(&config.server.oauth2.github);
    OAuth2Clients { github, google }
}

fn init_oauth2_client(oauth2_config: &OAuth2Item) -> BasicClient {
    let client_id = ClientId::new(oauth2_config.client_id.clone());
    let client_secret = ClientSecret::new(oauth2_config.client_secret.clone());
    let auth_url = AuthUrl::new(oauth2_config.auth_url.clone()).unwrap();
    let token_url = TokenUrl::new(oauth2_config.token_url.clone()).unwrap();
    let client = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url));
    client.set_redirect_uri(RedirectUrl::new(oauth2_config.redirect_url.clone()).unwrap())
}

pub async fn start(config: Config) {
    // init search ip region xdb
    searcher_init(Some(config.db.xdb.clone()));
    let state = AppState::new(&config).await;
    let app = routes::app_routes(state.clone());
    let listener = tokio::net::TcpListener::bind(&config.server.server_url())
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
