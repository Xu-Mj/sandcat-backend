// 目前主要是两块配置：服务地址以及数据库地址

use dotenvy::dotenv;
use std::env;
use tokio::sync::OnceCell;

pub struct Config {
    server: ServerConfig,
    db: DatabaseConfig,
}

pub struct ServerConfig {
    host: String,
    port: u16,
}

pub struct DatabaseConfig {
    url: String,
}

impl Config {
    pub fn db_url(&self) -> &str {
        &self.db.url
    }

    pub fn server_host(&self) -> &str {
        &self.server.host
    }

    pub fn server_port(&self) -> u16 {
        self.server.port
    }
}

// 创建一个单例对象
pub static CONFIG: OnceCell<Config> = OnceCell::const_new();

pub async fn init_conf() -> Config {
    // 加载.env配置文件
    dotenv().ok();
    // 读取server配置项
    let server = ServerConfig {
        host: env::var("host").unwrap_or_else(|_| String::from("127.0.0.1")),
        port: env::var("port")
            .unwrap_or_else(|_| String::from("3000"))
            .parse::<u16>()
            .unwrap(),
    };
    let db = DatabaseConfig {
        url: env::var("DATABASE_URL").expect("DATABASE URL NEEDED"),
    };
    Config { server, db }
}

pub async fn config() -> &'static Config {
    CONFIG.get_or_init(init_conf).await
}
