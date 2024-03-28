pub mod custom_extract;
pub mod mongodb_tester;
mod service_register_center;
pub mod sqlx_tester;

use abi::errors::Error;
pub use service_register_center::*;

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
