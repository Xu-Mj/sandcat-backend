use abi::errors::Error;
use async_trait::async_trait;
use std::collections::HashSet;
use std::net::SocketAddr;

pub trait ServiceObject {
    fn name(&self) -> String;
    fn host(&self) -> String;
    fn port(&self) -> u16;
}

#[async_trait]
pub trait ServiceFetcher: Send + Sync {
    async fn fetch(&self) -> Result<HashSet<SocketAddr>, Error>;
}
