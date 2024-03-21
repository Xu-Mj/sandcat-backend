// db config
// server config

use crate::errors::Error;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    // db config
    pub db: DbConfig,
    // server config
    pub server: ServerConfig,
    pub kafka: KafkaConfig,
    pub redis: RedisConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisConfig {
    host: String,
    port: u16,
}

impl RedisConfig {
    pub fn url(&self) -> String {
        format!("redis://{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KafkaConfig {
    pub hosts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
    #[serde(default = "default_conn")]
    pub max_connections: u32,
}

fn default_conn() -> u32 {
    5
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn load(filename: impl AsRef<Path>) -> Result<Self, Error> {
        let content = fs::read_to_string(filename).map_err(|_| Error::ConfigReadError)?;
        serde_yaml::from_str(&content).map_err(|_| Error::ConfigParseError)
    }
}

impl DbConfig {
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

impl ServerConfig {
    pub fn url(&self, https: bool) -> String {
        if https {
            format!("https://{}:{}", self.host, self.port)
        } else {
            format!("http://{}:{}", self.host, self.port)
        }
    }

    pub fn with_port(&self, port: u16) -> ServerConfig {
        ServerConfig {
            host: self.host.clone(),
            port,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load() {
        let config = Config::load("./fixtures/im.yml").unwrap();
        println!("{:?}", config);
        assert_eq!(config.db.host, "localhost");
        assert_eq!(config.db.port, 5432);
        assert_eq!(config.db.user, "postgres");
        assert_eq!(config.db.password, "root");
    }
}
