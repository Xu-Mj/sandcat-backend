use std::{path::Path, thread};

use sqlx::PgPool;
use tokio::runtime::Runtime;

/// create a struct which has ability to create database automatically and drop it automatically when it is dropped.
pub struct TestDb {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: String,
}

/// format url
impl TestDb {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
        password: impl Into<String>,
        migrations: impl Into<String>,
    ) -> TestDb {
        let mut tdb = TestDb {
            host: host.into(),
            port,
            user: user.into(),
            password: password.into(),
            dbname: "".into(),
        };
        let server_url = tdb.server_url();
        let uuid = uuid::Uuid::new_v4();
        let dbname = format!("test_{}", uuid);
        tdb.dbname = dbname.clone();
        let url = tdb.url();
        let migrations = migrations.into();
        // create database in tokio runtime
        thread::spawn(move || {
            Runtime::new().unwrap().block_on(async move {
                let conn = PgPool::connect(&server_url).await.unwrap();

                // create database for test
                sqlx::query(&format!(r#"CREATE DATABASE "{}""#, dbname))
                    .execute(&conn)
                    .await
                    .unwrap();

                // run migrations
                let conn = PgPool::connect(&url).await.unwrap();
                sqlx::migrate::Migrator::new(Path::new(&migrations))
                    .await
                    .unwrap()
                    .run(&conn)
                    .await
                    .unwrap();
            });
        })
        .join()
        .unwrap();
        tdb
    }

    pub fn server_url(&self) -> String {
        if self.password.is_empty() {
            format!("postgres://{}@{}:{}", self.user, self.host, self.port)
        } else {
            format!(
                "postgres://{}:{}@{}:{}",
                self.user, self.password, self.host, self.port
            )
        }
    }

    pub fn url(&self) -> String {
        format!("{}/{}", self.server_url(), self.dbname)
    }

    pub async fn pool(&self) -> PgPool {
        sqlx::Pool::connect(&self.url()).await.unwrap()
    }

    pub fn dbname(&self) -> String {
        self.dbname.clone()
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let server_url = self.server_url();
        let dbname = self.dbname.clone();
        thread::spawn(move || {
            Runtime::new().unwrap().block_on(async move {
                let conn = PgPool::connect(&server_url).await.unwrap();
                // close other connections
                sqlx::query(&format!(r#"SELECT pg_terminate_backend(pg_stat_activity.pid) FROM pg_stat_activity WHERE pg_stat_activity.datname = '{dbname}' AND pid <> pg_backend_pid();"#))
                    .execute(&conn)
                    .await
                    .unwrap();
                sqlx::query(&format!(r#"DROP DATABASE "{dbname}""#))
                    .execute(&conn)
                    .await
                    .unwrap();
            });
        }).join().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::TestDb;

    #[tokio::test]
    async fn it_works() {
        let tdb = TestDb::new("localhost", 5432, "postgres", "postgres", "./migrations");
        sqlx::query("INSERT INTO todos (id, title) VALUES (1, 'test');")
            .execute(&tdb.pool().await)
            .await
            .unwrap();
    }
}
