use std::collections::HashMap;

use axum::async_trait;
use tracing::debug;

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
            .no_proxy()
            .build()
            .unwrap();
        Self { options, client }
    }

    pub fn api_url(&self, name: &str) -> String {
        self.url("agent", name)
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
        let response = self.client.put(&url).json(&registration).send().await?;
        debug!("register service: {:?} to consul{url}", registration);
        if !response.status().is_success() {
            return Err(Error::InternalServer(
                response.text().await.unwrap_or_default(),
            ));
        }
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
        let response = self.client.put(url).send().await?;
        if !response.status().is_success() {
            return Err(Error::InternalServer(
                response.text().await.unwrap_or_default(),
            ));
        }
        Ok(())
    }

    async fn filter_by_name(&self, name: &str) -> Result<Services, Error> {
        let url = self.api_url("services");
        let mut map = HashMap::new();
        map.insert("filter", format!("Service == {}", name));

        let services = self
            .client
            .get(url)
            .query(&map)
            .send()
            .await?
            .json::<Services>()
            .await?;
        Ok(services)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_deregister_should_work() {
        let config = Config::load("../abi/fixtures/im.yml").unwrap();

        let consul = Consul::from_config(&config);
        let registration = Registration {
            id: "test".to_string(),
            name: "test".to_string(),
            address: "127.0.0.1".to_string(),
            port: 8081,
            tags: vec!["test".to_string()],
            check: None,
        };
        let result = consul.register(registration).await;
        assert!(result.is_ok());
        // delete it
        let result = consul.deregister("test").await;
        assert!(result.is_ok());
    }

    // #[tokio::test]
    // async fn discovery_should_work() {
    //     let config = Config::load("../abi/fixtures/im.yml").unwrap();
    //
    //     let consul = Consul::from_config(&config);
    //     let registration = Registration {
    //         id: "test".to_string(),
    //         name: "test".to_string(),
    //         address: "127.0.0.1".to_string(),
    //         port: 8080,
    //         tags: vec!["test".to_string()],
    //         check: None,
    //     };
    //     let result = consul.register(registration).await;
    //     assert!(result.is_ok());
    //     // get it
    //     let result = consul.discovery().await;
    //     assert!(result.is_ok());
    //     assert!(result.unwrap().contains_key("test"));
    //
    //     let registration = Registration {
    //         id: "example".to_string(),
    //         name: "example".to_string(),
    //         address: "127.0.0.1".to_string(),
    //         port: 8081,
    //         tags: vec!["example".to_string()],
    //         check: None,
    //     };
    //     let result = consul.register(registration).await;
    //     assert!(result.is_ok());
    //     let result = consul.filter_by_name("example").await;
    //     println!("{:?}", result);
    //     assert!(result.is_ok());
    //     let result = result.unwrap();
    //     assert_eq!(result.len(), 1);
    //     assert!(result.contains_key("example"));
    //     // delete it
    //     // let result = consul.deregister("test").await;
    //     // assert!(result.is_ok());
    // }
}
