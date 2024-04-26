mod strategy;

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use tracing::debug;

use crate::api_utils::lb::strategy::{get_strategy, LoadBalanceStrategy, LoadBalanceStrategyType};
use utils::ServiceRegister;

/// load balancer
/// get the service address from consul
#[derive(Debug, Clone)]
pub struct LoadBalancer {
    /// service name in consul
    service_name: String,
    /// register center
    service_register: Arc<dyn ServiceRegister>,
    /// service set
    service_set: Arc<RwLock<BTreeSet<String>>>,
    /// load balance strategy
    strategy: Arc<dyn LoadBalanceStrategy>,
}

const UPDATE_SERVICE_INTERVAL: u64 = 10;

impl LoadBalancer {
    pub async fn new(
        service_name: String,
        lb_type: impl Into<LoadBalanceStrategyType>,
        service_register: Arc<dyn ServiceRegister>,
    ) -> Self {
        let strategy = get_strategy(lb_type.into());
        let mut balancer = Self {
            service_name,
            service_register,
            strategy,
            service_set: Arc::new(RwLock::new(BTreeSet::new())),
        };

        balancer.update().await;
        // update the service address every 10 seconds
        let mut cloned_balancer = balancer.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(UPDATE_SERVICE_INTERVAL)).await;
                cloned_balancer.update().await;
                debug!(
                    "update the service address in load balancer: {:?}",
                    cloned_balancer.service_set
                );
            }
        });
        balancer
    }

    pub async fn get_service(&self) -> Option<String> {
        let services = self.service_set.read().unwrap();
        let services_count = services.len();

        if services_count == 0 {
            None
        } else {
            // add counter and get the index
            let counter = self.strategy.index(services_count);
            let index = counter % services_count;

            // reset the counter when the index is 0
            // if index == 0 {
            //     Arc::get_mut(&mut self.strategy).unwrap().reset();
            // }
            services.iter().nth(index).cloned()
        }
    }

    /// update the service address
    async fn update(&mut self) {
        let services = self
            .service_register
            .filter_by_name(&self.service_name)
            .await
            .unwrap();
        let service_set = services
            .values()
            .map(|v| format!("{}:{}", v.address, v.port))
            .collect();

        let old_service_set = self.service_set.read().unwrap();
        // compare the new service set with the old one
        if *old_service_set == service_set {
            return;
        }

        drop(old_service_set);

        // update the service set
        let mut old_service_set = self.service_set.write().unwrap();
        // union the new service set with the old one
        *old_service_set = old_service_set.union(&service_set).cloned().collect();
    }
}
