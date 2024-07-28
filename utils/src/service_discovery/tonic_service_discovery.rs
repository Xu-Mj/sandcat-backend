use std::collections::HashSet;
use std::net::SocketAddr;
use std::task::{Context, Poll};

use tokio::sync::mpsc;
use tonic::body::BoxBody;
use tonic::client::GrpcService;
use tonic::transport::{Channel, Endpoint};
use tower::discover::Change;
use tracing::{error, warn};

use abi::errors::Error;

use crate::service_discovery::service_fetcher::ServiceFetcher;

/// custom load balancer for tonic
#[derive(Debug, Clone)]
pub struct LbWithServiceDiscovery(pub Channel);

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

pub struct DynamicServiceDiscovery<Fetcher: ServiceFetcher> {
    services: HashSet<SocketAddr>,
    sender: mpsc::Sender<Change<SocketAddr, Endpoint>>,
    dis_interval: tokio::time::Duration,
    service_center: Fetcher,
    schema: String,
}

impl<Fetcher: ServiceFetcher> DynamicServiceDiscovery<Fetcher> {
    pub fn new(
        service_center: Fetcher,
        dis_interval: tokio::time::Duration,
        sender: mpsc::Sender<Change<SocketAddr, Endpoint>>,
        schema: String,
    ) -> Self {
        Self {
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
        let x = self.service_center.fetch().await?;
        let change_set = self.change_set(&x).await;
        for change in change_set {
            self.sender.send(change).await.map_err(Error::internal)?;
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

    pub async fn run(mut self) {
        loop {
            tokio::time::sleep(self.dis_interval).await;
            // get services from service register center
            if let Err(e) = self.discovery().await {
                error!("discovery error:{:?}", e);
            }
        }
    }
}
