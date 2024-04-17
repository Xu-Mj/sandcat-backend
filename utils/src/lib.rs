use abi::config::Config;
use abi::errors::Error;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use tonic::transport::{Channel, Endpoint};

pub use service_register_center::*;

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
