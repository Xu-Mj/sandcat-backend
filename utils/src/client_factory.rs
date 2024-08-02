use abi::message::{chat_service_client::ChatServiceClient, msg_service_client::MsgServiceClient};

use crate::service_discovery::LbWithServiceDiscovery;

pub trait ClientFactory {
    // 可能的共有方法，正示例
    fn n(channel: LbWithServiceDiscovery) -> Self;
}

impl ClientFactory for ChatServiceClient<LbWithServiceDiscovery> {
    fn n(channel: LbWithServiceDiscovery) -> Self {
        // 实现细节...
        Self::new(channel)
    }
}

impl ClientFactory for MsgServiceClient<LbWithServiceDiscovery> {
    fn n(channel: LbWithServiceDiscovery) -> Self {
        // 实现细节...
        Self::new(channel)
    }
}
