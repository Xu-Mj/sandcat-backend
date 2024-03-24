use abi::config::Config;
use abi::errors::Error;
use abi::message::db_service_client::DbServiceClient;
use abi::message::{Msg, MsgToDb, SaveMessageRequest};
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::util::get_rdkafka_version;
use rdkafka::{ClientConfig, Message};
use tonic::transport::Channel;
use tracing::{debug, error};

pub struct ConsumerService {
    consumer: StreamConsumer,
    /// rpc client
    db_rpc: DbServiceClient<Channel>,
}

impl ConsumerService {
    pub async fn new(config: &Config) -> Self {
        // 打印 rdkafka 版本信息
        let (version_n, version_s) = get_rdkafka_version();
        println!("rdkafka version: {}, {}", version_n, version_s);

        // init kafka consumer
        // 创建 Kafka 配置
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &config.kafka.group)
            .set("bootstrap.servers", config.kafka.hosts.join(","))
            .set("enable.auto.commit", "true")
            .set("auto.commit.interval.ms", "1000")
            .set("session.timeout.ms", "6000")
            .set("enable.partition.eof", "false")
            .create()
            .expect("Consumer creation failed");

        // 订阅主题
        consumer
            .subscribe(&[&config.kafka.topic])
            .expect("Can't subscribe to specified topic");

        // init rpc client
        let db_rpc = DbServiceClient::connect(config.rpc.db.url(false))
            .await
            .unwrap();
        Self { consumer, db_rpc }
    }

    pub async fn consume(&mut self) -> Result<(), Error> {
        let mut db_rpc = self.db_rpc.clone();
        // 开始消费消息
        loop {
            match self.consumer.recv().await {
                Err(e) => error!("Kafka error: {}", e),
                Ok(m) => {
                    if let Some(payload) = m.payload() {
                        debug!(
                            "Received message: {:#?}",
                            String::from_utf8(payload.to_vec())
                        );

                        if let Err(e) = Self::send_to_db(&mut db_rpc, payload).await {
                            error!("failed to consume message, error: {:?}", e);
                            continue;
                        }

                        if let Err(e) = self.consumer.commit_message(&m, CommitMode::Async) {
                            error!("Failed to commit message: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    pub async fn send_to_db(
        db_rpc: &mut DbServiceClient<Channel>,
        msg: &[u8],
    ) -> Result<(), Error> {
        let msg: Msg = serde_json::from_slice(msg)?;
        db_rpc
            .save_message(SaveMessageRequest {
                message: Some(MsgToDb::from(msg)),
            })
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;
        Ok(())
    }
}
