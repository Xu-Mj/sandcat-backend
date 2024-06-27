// db config
// server config

use crate::errors::Error;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub component: Component,
    // db config
    pub db: DbConfig,
    // server config
    pub server: ServerConfig,
    pub kafka: KafkaConfig,
    pub redis: RedisConfig,
    pub rpc: RpcConfig,
    pub websocket: WsServerConfig,
    pub service_center: ServiceCenterConfig,
    pub oss: OssConfig,
    pub mail: MailConfig,
    pub log: LogConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Component {
    Chat,
    Consumer,
    Db,
    Pusher,
    Ws,
    All,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogConfig {
    pub level: String,
    pub output: String,
}

impl LogConfig {
    pub fn level(&self) -> tracing::Level {
        match self.level.as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MailConfig {
    pub server: String,
    pub account: String,
    pub password: String,
    pub temp_path: String,
    pub temp_file: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OssConfig {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    pub avatar_bucket: String,
    pub region: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceCenterConfig {
    pub host: String,
    pub port: u16,
    pub protocol: String,
    pub timeout: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DbConfig {
    // db config
    pub postgres: PostgresConfig,
    pub mongodb: MongoDbConfig,
    pub xdb: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RpcConfig {
    pub health_check: bool,
    pub ws: RpcServerConfig,
    pub chat: RpcServerConfig,
    pub db: RpcServerConfig,
    pub pusher: RpcServerConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PusherRpcServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WsServerConfig {
    pub protocol: String,
    pub host: String,
    pub port: u16,
    pub name: String,
    pub tags: Vec<String>,
}

impl WsServerConfig {
    #[inline]
    pub fn rpc_server_url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    #[inline]
    pub fn url(&self, https: bool) -> String {
        url(https, &self.host, self.port)
    }

    pub fn ws_url(&self, https: bool) -> String {
        if https {
            format!("wss://{}:{}", self.host, self.port)
        } else {
            format!("ws://{}:{}", self.host, self.port)
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RpcServerConfig {
    pub protocol: String,
    pub host: String,
    pub port: u16,
    pub name: String,
    pub tags: Vec<String>,
    pub grpc_health_check: GrpcHealthCheck,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrpcHealthCheck {
    pub grpc_use_tls: bool,
    pub interval: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatRpcServerConfig {
    pub host: String,
    pub port: u16,
}

impl PusherRpcServerConfig {
    #[inline]
    pub fn rpc_server_url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    #[inline]
    pub fn url(&self, https: bool) -> String {
        url(https, &self.host, self.port)
    }
}

impl RpcServerConfig {
    #[inline]
    pub fn rpc_server_url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    #[inline]
    pub fn url(&self, https: bool) -> String {
        url(https, &self.host, self.port)
    }
}

impl ChatRpcServerConfig {
    #[inline]
    pub fn rpc_server_url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    #[inline]
    pub fn url(&self, https: bool) -> String {
        url(https, &self.host, self.port)
    }
}

fn url(https: bool, host: &str, port: u16) -> String {
    if https {
        format!("https://{}:{}", host, port)
    } else {
        format!("http://{}:{}", host, port)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub seq_step: i32,
}

impl RedisConfig {
    pub fn url(&self) -> String {
        format!("redis://{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KafkaConfig {
    pub hosts: Vec<String>,
    pub topic: String,
    pub connect_timeout: u16,
    pub group: String,
    pub producer: KafkaProducer,
    pub consumer: KafkaConsumer,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KafkaProducer {
    pub timeout: u16,
    pub acks: String,
    pub max_retry: u8,
    pub retry_interval: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KafkaConsumer {
    pub session_timeout: u16,
    pub auto_offset_reset: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
    #[serde(default = "default_conn")]
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MongoDbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

fn default_conn() -> u32 {
    5
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub ws_lb_strategy: String,
    pub oauth2: Vec<OAuth2>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuth2 {
    pub tp: OAuth2Type,
    pub client_id: String,
    pub client_secret: String,
    // pub redirect_url: String,
    pub auth_url: String,
    pub token_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuth2Type {
    Google,
    Github,
}

impl Config {
    pub fn load(filename: impl AsRef<Path>) -> Result<Self, Error> {
        let content = fs::read_to_string(filename).map_err(|_| Error::ConfigReadError)?;
        serde_yaml::from_str(&content).map_err(Error::ConfigParseError)
    }
}

impl PostgresConfig {
    pub fn server_url(&self) -> String {
        if self.password.is_empty() {
            return format!("postgres://{}@{}:{}", self.user, self.host, self.port);
        }
        format!(
            "postgres://{}:{}@{}:{}",
            self.user, self.password, self.host, self.port
        )
    }
    pub fn url(&self) -> String {
        format!("{}/{}", self.server_url(), self.database)
    }
}

impl MongoDbConfig {
    pub fn server_url(&self) -> String {
        match (self.user.is_empty(), self.password.is_empty()) {
            (true, _) => {
                format!("mongodb://{}:{}", self.host, self.port)
            }
            (false, true) => {
                format!("mongodb://{}@{}:{}", self.user, self.host, self.port)
            }
            (false, false) => {
                format!(
                    "mongodb://{}:{}@{}:{}",
                    self.user, self.password, self.host, self.port
                )
            }
        }
    }
    pub fn url(&self) -> String {
        format!("{}/{}", self.server_url(), self.database)
    }
}

impl ServerConfig {
    pub fn url(&self, https: bool) -> String {
        url(https, &self.host, self.port)
    }
    pub fn server_url(&self) -> String {
        format!("{}:{}", &self.host, self.port)
    }

    pub fn with_port(&self, port: u16) -> ServerConfig {
        ServerConfig {
            host: self.host.clone(),
            port,
            jwt_secret: self.jwt_secret.clone(),
            ws_lb_strategy: self.ws_lb_strategy.clone(),
            oauth2: self.oauth2.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load() {
        let config = match Config::load("../config.yml") {
            Ok(config) => config,
            Err(err) => {
                panic!("load config error: {:?}", err);
            }
        };
        println!("{:?}", config);
        assert_eq!(config.db.postgres.host, "localhost");
        assert_eq!(config.db.postgres.port, 5432);
        assert_eq!(config.db.postgres.user, "postgres");
        assert_eq!(config.db.postgres.password, "postgres");
    }
}
