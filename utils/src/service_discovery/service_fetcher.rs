use abi::errors::Error;
use async_trait::async_trait;
use std::collections::HashSet;
use std::net::SocketAddr;

#[async_trait]
pub trait ServiceFetcher: Send + Sync {
    async fn fetch(&self) -> Result<HashSet<SocketAddr>, Error>;
}
