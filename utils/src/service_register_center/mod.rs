use crate::service_register_center::typos::{Registration, Service};
use abi::config::Config;
use abi::errors::Error;
use axum::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

mod consul;
mod tonic_service_discovery;
pub mod typos;
pub use tonic_service_discovery::*;

pub type Services = HashMap<String, Service>;
/// the service register discovery center
#[async_trait]
pub trait ServiceRegister: Send + Sync + Debug {
    /// service register
    async fn register(&self, registration: Registration) -> Result<(), Error>;

    /// service discovery
    async fn discovery(&self) -> Result<Services, Error>;

    /// service deregister
    async fn deregister(&self, service_id: &str) -> Result<(), Error>;

    /// filter
    async fn filter_by_name(&self, name: &str) -> Result<Services, Error>;
}

pub fn service_register_center(config: &Config) -> Arc<dyn ServiceRegister> {
    Arc::new(consul::Consul::from_config(config))
}
