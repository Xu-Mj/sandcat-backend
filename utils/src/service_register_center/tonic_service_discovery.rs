use crate::{service_register_center, ServiceRegister};
use abi::config::Config;
use abi::errors::Error;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use tonic::body::BoxBody;
use tonic::client::GrpcService;
use tonic::transport::{Channel, Endpoint};
use tower::discover::Change;
use tracing::{error, warn};

/// custom load balancer for tonic
#[derive(Debug, Clone)]
pub struct LbWithServiceDiscovery(Channel);

/// implement the tonic service for custom load balancer
impl tower::Service<http::Request<BoxBody>> for LbWithServiceDiscovery {
    type Response = http::Response<<Channel as GrpcService<BoxBody>>::ResponseBody>;
    type Error = <Channel as GrpcService<BoxBody>>::Error;
    type Future = <Channel as GrpcService<BoxBody>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        GrpcService::poll_ready(&mut self.0, cx)
    }

    fn call(&mut self, request: http::Request<BoxBody>) -> Self::Future {
        GrpcService::call(&mut self.0, request)
    }
}

pub struct DynamicServiceDiscovery {
    service_name: String,
    services: HashSet<SocketAddr>,
    sender: mpsc::Sender<Change<SocketAddr, Endpoint>>,
    dis_interval: tokio::time::Duration,
    service_center: Arc<dyn ServiceRegister>,
    schema: String,
}

impl DynamicServiceDiscovery {
    pub fn with_config(
        config: &Config,
        service_name: String,
        dis_interval: tokio::time::Duration,
        sender: mpsc::Sender<Change<SocketAddr, Endpoint>>,
        schema: String,
    ) -> Self {
        let service_center = service_register_center(config);
        Self {
            service_name,
            services: Default::default(),
            sender,
            dis_interval,
            service_center,
            schema,
        }
    }

    pub fn new(
        service_center: Arc<dyn ServiceRegister>,
        service_name: String,
        dis_interval: tokio::time::Duration,
        sender: mpsc::Sender<Change<SocketAddr, Endpoint>>,
        schema: String,
    ) -> Self {
        Self {
            service_name,
            services: Default::default(),
            sender,
            dis_interval,
            service_center,
            schema,
        }
    }

    /// execute discovery once
    pub async fn discovery(&mut self) -> Result<(), Error> {
        //get services from service register center
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
        let change_set = self.change_set(&x).await;
        for change in change_set {
            self.sender
                .send(change)
                .await
                .map_err(|e| Error::InternalServer(e.to_string()))?;
        }
        self.services = x;
        Ok(())
    }

    async fn change_set(
        &self,
        endpoints: &HashSet<SocketAddr>,
    ) -> Vec<Change<SocketAddr, Endpoint>> {
        let mut changes = Vec::new();
        for s in endpoints.difference(&self.services) {
            if let Some(endpoint) = self.build_endpoint(*s).await {
                changes.push(Change::Insert(*s, endpoint));
            }
        }
        for s in self.services.difference(endpoints) {
            changes.push(Change::Remove(*s));
        }
        changes
    }

    async fn build_endpoint(&self, address: SocketAddr) -> Option<Endpoint> {
        let url = format!("{}://{}:{}", self.schema, address.ip(), address.port());
        let endpoint = Endpoint::from_shared(url)
            .map_err(|e| warn!("build endpoint error:{:?}", e))
            .ok()?;
        Some(endpoint)
    }

    pub async fn run(mut self) -> Result<(), Error> {
        loop {
            tokio::time::sleep(self.dis_interval).await;
            // get services from service register center
            if let Err(e) = self.discovery().await {
                error!("discovery error:{}", e);
            }
        }
    }
}

pub async fn get_channel_with_config(
    config: &Config,
    service_name: impl ToString,
    protocol: impl ToString,
) -> Result<LbWithServiceDiscovery, Error> {
    let (channel, sender) = Channel::balance_channel(1024);
    let discovery = DynamicServiceDiscovery::with_config(
        config,
        service_name.to_string(),
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
    let discovery = DynamicServiceDiscovery::new(
        register,
        service_name.to_string(),
        tokio::time::Duration::from_secs(10),
        sender,
        protocol.to_string(),
    );
    get_channel(discovery, channel).await
}

async fn get_channel(
    mut discovery: DynamicServiceDiscovery,
    channel: Channel,
) -> Result<LbWithServiceDiscovery, Error> {
    discovery.discovery().await?;
    tokio::spawn(discovery.run());
    Ok(LbWithServiceDiscovery(channel))
}
