use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use async_trait::async_trait;
use client_factory::ClientFactory;
use synapse::health::HealthCheck;
use synapse::service::client::ServiceClient;
use synapse::service::{
    Scheme, ServiceInstance, ServiceRegistryClient, ServiceRegistryServer, ServiceStatus,
};
use tokio::sync::mpsc::Sender;
use tonic::transport::{Channel, Endpoint, Server};
use tower::discover::Change;
use tracing::log::warn;
use tracing::{debug, error};

use crate::service_discovery::{DynamicServiceDiscovery, LbWithServiceDiscovery, ServiceFetcher};
use abi::config::{Component, Config};
use abi::errors::Error;

pub use service_register_center::*;

mod client_factory;
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
    let mut client = ServiceClient::builder()
        .server_host(config.service_center.host.clone())
        .server_port(config.service_center.port)
        .connect_timeout(Duration::from_secs(5))
        .build()
        .await
        .map_err(|_| {
            Error::InternalServer("Connect to service register center failed".to_string())
        })?;

    tokio::spawn(async move {
        let mut stream = match client.subscribe(name).await {
            Ok(stream) => stream,
            Err(e) => {
                error!("subscribe channel error: {:?}", e);
                return;
            }
        };
        while let Some(service) = stream.recv().await {
            debug!("subscribe channel return: {:?}", service);
            let addr = format!("{}:{}", service.address, service.port);
            let socket_addr: SocketAddr = addr.parse().unwrap();
            let scheme = Scheme::from(service.scheme as u8);
            let addr = format!("{}://{}", scheme, addr);
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

pub async fn start_register_center(config: &Config) {
    let hub = synapse::service::hub::Hub::new();
    let server = ServiceRegistryServer::new(hub);
    let addr = format!(
        "{}:{}",
        config.service_center.host, config.service_center.port
    );
    Server::builder()
        .add_service(server)
        .serve(addr.parse().unwrap())
        .await
        .unwrap();
}

pub async fn register_service(config: &Config, com: Component) -> Result<(), Error> {
    // register service to service register center
    let addr = format!(
        "{}://{}:{}",
        config.service_center.protocol, config.service_center.host, config.service_center.port
    );
    let endpoint = Endpoint::from_shared(addr)
        .map_err(|e| Error::TonicError(e.to_string()))?
        .connect_timeout(Duration::from_secs(config.service_center.timeout));
    let mut client = ServiceRegistryClient::connect(endpoint)
        .await
        .map_err(|e| Error::TonicError(e.to_string()))?;

    let (scheme, name, host, port, tags) = match com {
        abi::config::Component::Chat => {
            let scheme = Scheme::from(config.rpc.chat.protocol.as_str()) as i32;
            let name = config.rpc.chat.name.clone();
            let host = config.rpc.chat.host.clone();
            let port = config.rpc.chat.port as i32;
            let tags = config.rpc.chat.tags.clone();
            (scheme, name, host, port, tags)
        }
        abi::config::Component::Consumer => todo!("consumer"),
        abi::config::Component::Db => {
            let scheme = Scheme::from(config.rpc.db.protocol.as_str()) as i32;
            let name = config.rpc.db.name.clone();
            let host = config.rpc.db.host.clone();
            let port = config.rpc.db.port as i32;
            let tags = config.rpc.db.tags.clone();
            (scheme, name, host, port, tags)
        }
        abi::config::Component::Pusher => {
            let scheme = Scheme::from(config.rpc.pusher.protocol.as_str()) as i32;
            let name = config.rpc.pusher.name.clone();
            let host = config.rpc.pusher.host.clone();
            let port = config.rpc.pusher.port as i32;
            let tags = config.rpc.pusher.tags.clone();
            (scheme, name, host, port, tags)
        }
        abi::config::Component::Ws => {
            let scheme = Scheme::from(config.rpc.ws.protocol.as_str()) as i32;
            let name = config.rpc.ws.name.clone();
            let host = config.rpc.ws.host.clone();
            let port = config.rpc.ws.port as i32;
            let tags = config.rpc.ws.tags.clone();
            (scheme, name, host, port, tags)
        }
        abi::config::Component::All => todo!("all"),
    };

    let mut health_check = None;
    if config.rpc.health_check {
        health_check = Some(HealthCheck {
            endpoint: "".to_string(),
            interval: 10,
            timeout: 10,
            retries: 10,
            scheme,
            tls_domain: None,
        });
    }
    let service = ServiceInstance {
        id: format!("{}-{}", get_host_name()?, name),
        name,
        address: host,
        port,
        tags,
        version: "".to_string(),
        metadata: Default::default(),
        health_check,
        status: 0,
        scheme,
    };
    client.register_service(service).await.unwrap();
    Ok(())
}

pub async fn get_rpc_client<T: ClientFactory>(
    config: &Config,
    service_name: String,
) -> Result<T, Error> {
    let channel = get_chan(config, service_name).await?;
    Ok(T::n(channel))
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
