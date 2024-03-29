use abi::config::Config;
use abi::errors::Error;
use tonic::transport::{Channel, Endpoint};

pub use service_register_center::*;

pub mod custom_extract;
pub mod mongodb_tester;
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
    let mut ws_list = crate::service_register_center(config)
        .filter_by_name(name)
        .await?;

    // retry 5 times if no ws rpc url
    if ws_list.is_empty() {
        for i in 0..5 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            ws_list = crate::service_register_center(config)
                .filter_by_name(name)
                .await?;
            if !ws_list.is_empty() {
                break;
            }
            if i == 5 {
                return Err(Error::ServiceNotFound(String::from(name)));
            }
        }
    }
    let endpoints = ws_list.values().map(|v| {
        let url = format!("{}://{}:{}", protocol, v.address, v.port);
        Endpoint::from_shared(url).unwrap()
    });
    let channel = Channel::balance_list(endpoints);
    Ok(channel)
}
