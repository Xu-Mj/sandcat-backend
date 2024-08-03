use abi::config::Config;
use tracing::{error, info};

// 缓存中设置一个标志，用来标识是否已经加载过用户的max_seq
pub async fn load_seq(config: &Config) {
    info!("check is loaded");
    let redis = cache::cache(config);

    let flag = redis.check_seq_loaded().await.unwrap();
    if !flag {
        info!("seq already loaded");
        return;
    }

    info!("seq not loaded");
    info!("loading seq start...");
    let db_repo = db::DbRepo::new(config).await;
    let mut rx = db_repo.seq.get_max_seq().await.unwrap();
    let batch_size = 50;
    let mut list = Vec::with_capacity(batch_size);
    while let Some(item) = rx.recv().await {
        list.push(item);
        if list.len() == batch_size {
            if let Err(e) = redis.set_seq(&list).await {
                error!("set seq error: {:?}", e)
            };
            list.clear();
        }
    }

    if !list.is_empty() {
        if let Err(e) = redis.set_seq(&list).await {
            error!("set seq error: {:?}", e)
        };
        list.clear();
    }

    // set loaded flag
    redis.set_seq_loaded().await.unwrap();
}
