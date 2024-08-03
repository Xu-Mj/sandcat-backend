use abi::config::Config;
use consumer::ConsumerService;
use productor::ChatRpcService;

pub mod consumer;
pub mod productor;
mod pusher;

pub async fn start(config: &Config) {
    let cloned_conf = config.clone();
    let pro = tokio::spawn(async move {
        ChatRpcService::start(&cloned_conf).await;
    });

    let cloned_conf = config.clone();
    let con = tokio::spawn(async move {
        ConsumerService::new(&cloned_conf)
            .await
            .consume()
            .await
            .unwrap();
    });

    tokio::try_join!(pro, con).unwrap();
}
