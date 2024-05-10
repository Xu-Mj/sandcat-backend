use std::thread;

use mongodb::Database;
use tokio::runtime::Runtime;

pub struct MongoDbTester {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: String,
}

impl MongoDbTester {
    pub async fn new(
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> MongoDbTester {
        let uuid = uuid::Uuid::new_v4();
        let dbname = format!("test_{}", uuid);
        let tdb = MongoDbTester {
            host: host.into(),
            port,
            user: user.into(),
            password: password.into(),
            dbname: dbname.clone(),
        };
        let server_url = tdb.server_url();
        let client = mongodb::Client::with_uri_str(server_url).await.unwrap();
        client.database(&dbname);
        tdb
    }
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
        format!("{}/{}", self.server_url(), self.dbname)
    }

    pub fn dbname(&self) -> String {
        self.dbname.clone()
    }

    pub async fn database(&self) -> Database {
        mongodb::Client::with_uri_str(self.url())
            .await
            .unwrap()
            .database(&self.dbname)
    }
}

impl Drop for MongoDbTester {
    fn drop(&mut self) {
        let server_url = self.server_url();
        let dbname = self.dbname.clone();
        // drop database
        thread::spawn(move || {
            Runtime::new().unwrap().block_on(async move {
                let client = mongodb::Client::with_uri_str(server_url).await.unwrap();
                if let Err(e) = client.database(&dbname).drop(None).await {
                    println!("drop database error: {}", e);
                }
                println!("drop trait over{}", dbname);
            });
        })
        .join()
        .unwrap();
    }
}
