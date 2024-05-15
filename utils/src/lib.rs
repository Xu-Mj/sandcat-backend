use abi::config::Config;
use abi::errors::Error;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use async_trait::async_trait;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use synapse::pb::{QueryRequest, ServiceStatus, SubscribeRequest};
use tokio::sync::mpsc::Sender;
use tonic::transport::{Channel, Endpoint};
use tower::discover::Change;
use tracing::debug;
use tracing::log::warn;

use crate::service_discovery::{DynamicServiceDiscovery, LbWithServiceDiscovery, ServiceFetcher};
pub use service_register_center::*;

pub mod mongodb_tester;
pub mod service_discovery;
mod service_register_center;
pub mod sqlx_tester;

// get host name
pub fn get_host_name() -> Result<String, Error> {
    let hostname = hostname::get()?;
    let hostname = hostname.into_string().map_err(|_| {
        Error::InternalServer(String::from(
            "get hostname error: OsString into String Failed",
        ))
    })?;
    Ok(hostname)
}

pub async fn get_rpc_channel_by_name(
    config: &Config,
    name: &str,
    protocol: &str,
) -> Result<Channel, Error> {
    let center = crate::service_register_center(config);
    let mut service_list = center.filter_by_name(name).await?;

    // retry 5 times if no ws rpc url
    if service_list.is_empty() {
        for i in 0..5 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            service_list = center.filter_by_name(name).await?;
            if !service_list.is_empty() {
                break;
            }
            if i == 5 {
                return Err(Error::ServiceNotFound(String::from(name)));
            }
        }
    }
    let endpoints = service_list.values().map(|v| {
        let url = format!("{}://{}:{}", protocol, v.address, v.port);
        Endpoint::from_shared(url).unwrap()
    });
    let channel = Channel::balance_list(endpoints);
    Ok(channel)
}

pub fn generate_salt() -> String {
    SaltString::generate(&mut OsRng).to_string()
}

pub fn hash_password(password: &[u8], salt: &str) -> Result<String, Error> {
    // 使用默认的Argon2配置
    // 这个配置可以更改为适合您具体安全需求和性能要求的设置

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Hash password to PHC string ($argon2id$v=19$...)
    Ok(argon2
        .hash_password(password, &SaltString::from_b64(salt).unwrap())
        .map_err(|e| Error::InternalServer(e.to_string()))?
        .to_string())
}

pub struct ServiceResolver {
    service_name: String,
    service_center: Arc<dyn ServiceRegister>,
}

#[async_trait]
impl ServiceFetcher for ServiceResolver {
    async fn fetch(&self) -> Result<HashSet<SocketAddr>, Error> {
        let map = self
            .service_center
            .filter_by_name(&self.service_name)
            .await?;
        let x = map
            .values()
            .filter_map(|v| match format!("{}:{}", v.address, v.port).parse() {
                Ok(s) => Some(s),
                Err(e) => {
                    warn!("parse address error:{}", e);
                    None
                }
            })
            .collect();
        Ok(x)
    }
}

impl ServiceResolver {
    pub fn new(service_center: Arc<dyn ServiceRegister>, service_name: String) -> Self {
        Self {
            service_name,
            service_center,
        }
    }
}

pub async fn get_channel_with_config(
    config: &Config,
    service_name: impl ToString,
    protocol: impl ToString,
) -> Result<LbWithServiceDiscovery, Error> {
    let (channel, sender) = Channel::balance_channel(1024);
    let service_resolver = ServiceResolver::new(
        crate::service_register_center(config),
        service_name.to_string(),
    );
    let discovery = DynamicServiceDiscovery::new(
        service_resolver,
        tokio::time::Duration::from_secs(10),
        sender,
        protocol.to_string(),
    );
    get_channel(discovery, channel).await
}

pub async fn get_channel_with_register(
    register: Arc<dyn ServiceRegister>,
    service_name: impl ToString,
    protocol: impl ToString,
) -> Result<LbWithServiceDiscovery, Error> {
    let (channel, sender) = Channel::balance_channel(1024);
    let service_resolver = ServiceResolver::new(register, service_name.to_string());
    let discovery = DynamicServiceDiscovery::new(
        service_resolver,
        tokio::time::Duration::from_secs(10),
        sender,
        protocol.to_string(),
    );
    get_channel(discovery, channel).await
}

async fn get_channel(
    mut discovery: DynamicServiceDiscovery<ServiceResolver>,
    channel: Channel,
) -> Result<LbWithServiceDiscovery, Error> {
    discovery.discovery().await?;
    tokio::spawn(discovery.run());
    Ok(LbWithServiceDiscovery(channel))
}

pub async fn get_chan(config: &Config, name: String) -> Result<LbWithServiceDiscovery, Error> {
    let (channel, sender) = Channel::balance_channel(1024);
    get_chan_(config, name, sender).await?;
    Ok(LbWithServiceDiscovery(channel))
}

pub async fn get_chan_(
    config: &Config,
    name: String,
    sender: Sender<Change<SocketAddr, Endpoint>>,
) -> Result<(), Error> {
    let addr = format!(
        "{}://{}:{}",
        config.service_center.protocol, config.service_center.host, config.service_center.port
    );
    tokio::spawn(async move {
        let endpoint = Endpoint::from_shared(addr).unwrap();
        let mut client =
            synapse::pb::service_registry_client::ServiceRegistryClient::connect(endpoint)
                .await
                .unwrap();
        // query service list
        debug!("query service list");
        let res = client
            .query_services(QueryRequest { name: name.clone() })
            .await
            .unwrap();
        let services = res.into_inner().services;
        debug!("services:{:?}", services);
        for service in services {
            // todo need to modify service register center to add protocol attr
            let addr = format!("{}:{}", service.address, service.port);
            let socket_addr: SocketAddr = addr.parse().unwrap();
            let addr = format!("http://{}", addr);

            sender
                .send(Change::Insert(
                    socket_addr,
                    Endpoint::from_shared(addr).unwrap(),
                ))
                .await
                .unwrap();
        }

        let response = client
            .subscribe(SubscribeRequest { service: name })
            .await
            .unwrap();
        debug!("subscribe success");
        let mut stream = response.into_inner();
        while let Some(service) = stream.message().await.unwrap() {
            debug!("subscribe channel return: {:?}", service);
            let addr = format!("{}:{}", service.address, service.port);
            let socket_addr: SocketAddr = addr.parse().unwrap();
            let addr = format!("http://{}", addr);
            let change = if service.active == ServiceStatus::Up as i32 {
                let endpoint = Endpoint::from_shared(addr).unwrap();
                Change::Insert(socket_addr, endpoint)
            } else {
                Change::Remove(socket_addr)
            };
            sender.send(change).await.unwrap();
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::{PasswordHash, PasswordVerifier};

    #[test]
    fn test_hash_password() {
        let salt = generate_salt();
        let password = "123456";
        let hash = hash_password(password.as_bytes(), &salt).unwrap();
        let parsed_hash = PasswordHash::new(&hash).unwrap();
        assert!(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok());
    }
}
