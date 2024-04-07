use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Registration {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub tags: Vec<String>,
    pub check: Option<GrpcHealthCheck>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GrpcHealthCheck {
    pub name: String,
    pub grpc: String,
    pub grpc_use_tls: bool,
    pub interval: String,
}

/// returned type from register center
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Service {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Service")]
    pub service: String,
    #[serde(rename = "Address")]
    pub address: String,
    #[serde(rename = "Port")]
    pub port: u16,
    #[serde(rename = "Tags")]
    pub tags: Vec<String>,
    #[serde(rename = "Datacenter")]
    pub datacenter: String,
}
