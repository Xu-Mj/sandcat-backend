use axum::async_trait;

use abi::config::Config;
use abi::errors::Error;

use crate::service_register_center::typos::Registration;
use crate::service_register_center::{ServiceRegister, Services};

/// consul options
pub struct ConsulOptions {
    pub host: String,
    pub port: u16,
    pub protocol: String,
    pub timeout: u64,
}

impl ConsulOptions {
    #[allow(dead_code)]
    pub fn from_config(config: &Config) -> Self {
        Self {
            host: config.service_center.host.clone(),
            port: config.service_center.port,
            timeout: config.service_center.timeout,
            protocol: config.service_center.protocol.clone(),
        }
    }
}

pub struct Consul {
    pub options: ConsulOptions,
    pub client: reqwest::Client,
}

impl Consul {
    #[allow(dead_code)]
    pub fn from_config(config: &Config) -> Self {
        let options = ConsulOptions::from_config(config);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(options.timeout))
            .build()
            .unwrap();
        Self { options, client }
    }

    pub fn api_url(&self, name: &str) -> String {
        self.url("agent", name)
    }

    /// used by filter services by service name
    pub fn catalog_url(&self, name: &str) -> String {
        self.url("catalog", name)
    }

    fn url(&self, type_: &str, name: &str) -> String {
        format!(
            "{}://{}:{}/v1/{}/{}",
            self.options.protocol, self.options.host, self.options.port, type_, name
        )
    }
}

#[async_trait]
impl ServiceRegister for Consul {
    async fn register(&self, registration: Registration) -> Result<(), Error> {
        let url = self.api_url("service/register");
        self.client.put(url).json(&registration).send().await?;
        Ok(())
    }

    async fn discovery(&self) -> Result<Services, Error> {
        let url = self.api_url("services");
        let services = self
            .client
            .get(url)
            .send()
            .await?
            .json::<Services>()
            .await?;
        Ok(services)
    }

    async fn deregister(&self, service_id: &str) -> Result<(), Error> {
        let url = self.api_url(&format!("service/deregister/{}", service_id));
        self.client.put(url).send().await?;
        Ok(())
    }

    async fn filter_by_name(&self, name: &str) -> Result<Services, Error> {
        let url = self.catalog_url(&format!("service/{}", name));
        let services = self
            .client
            .get(url)
            .send()
            .await?
            .json::<Services>()
            .await?;
        Ok(services)
    }
}
