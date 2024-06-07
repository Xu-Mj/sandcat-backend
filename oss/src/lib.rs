use abi::config::Config;
use abi::errors::Error;
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

mod client;

#[async_trait]
pub trait Oss: Debug + Send + Sync {
    async fn file_exists(&self, key: &str, local_md5: &str) -> Result<bool, Error>;
    async fn upload_file(&self, key: &str, content: Vec<u8>) -> Result<(), Error>;
    async fn download_file(&self, key: &str) -> Result<Bytes, Error>;
    async fn delete_file(&self, key: &str) -> Result<(), Error>;

    async fn upload_avatar(&self, key: &str, content: Vec<u8>) -> Result<(), Error>;
    async fn download_avatar(&self, key: &str) -> Result<Bytes, Error>;
    async fn delete_avatar(&self, key: &str) -> Result<(), Error>;
}

pub async fn oss(config: &Config) -> Arc<dyn Oss> {
    Arc::new(client::S3Client::new(config).await)
}

pub fn default_avatars() -> HashMap<String, String> {
    HashMap::from([
        (
            String::from("./oss/default_avatar/avatar1.png"),
            String::from("avatar1.png"),
        ),
        (
            String::from("./oss/default_avatar/avatar2.png"),
            String::from("avatar2.png"),
        ),
        (
            String::from("./oss/default_avatar/avatar3.png"),
            String::from("avatar3.png"),
        ),
    ])
}
