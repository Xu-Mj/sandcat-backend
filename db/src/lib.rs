use tracing::info;

use abi::{config::Config, message::MsgType};

pub mod database;
pub mod rpc;

pub async fn clean_receive_box(config: &Config) {
    let types: Vec<i32> = config
        .db
        .mongodb
        .clean
        .except_types
        .iter()
        .filter_map(|v| MsgType::from_str_name(v))
        .map(|v| v as i32)
        .collect();
    let period = config.db.mongodb.clean.period;

    let msg_box = database::msg_rec_box_cleaner(config).await;
    msg_box.clean_receive_box(period, types);
    info!("clean receive box task started");
}
