use abi::config::Config;
use abi::errors::Error;
use abi::message::db_service_client::DbServiceClient;
use abi::message::msg::Data;
use abi::message::push_service_client::PushServiceClient;
use abi::message::{Msg, MsgToDb, SaveMessageRequest, SendMsgRequest};
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use tonic::transport::Channel;
use tracing::{debug, error};

pub struct ConsumerService {
    consumer: StreamConsumer,
    /// rpc client
    db_rpc: DbServiceClient<Channel>,
    pusher: PushServiceClient<Channel>,
}

impl ConsumerService {
    pub async fn new(config: &Config) -> Self {
        // init kafka consumer
        // todo 了解kafka相关配置
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &config.kafka.group)
            .set("bootstrap.servers", config.kafka.hosts.join(","))
            // .set("enable.auto.commit", "true")
            // .set("auto.commit.interval.ms", "1000")
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

        let pusher = PushServiceClient::connect(config.rpc.pusher.url(false))
            .await
            .unwrap();

        Self {
            consumer,
            db_rpc,
            pusher,
        }
    }

    pub async fn consume(&mut self) -> Result<(), Error> {
        // 开始消费消息
        loop {
            match self.consumer.recv().await {
                Err(e) => error!("Kafka error: {}", e),
                Ok(m) => {
                    if let Some(Ok(payload)) = m.payload_view::<str>() {
                        debug!("Received message: {:#?}", payload);

                        // send to db rpc server
                        let mut db_rpc = self.db_rpc.clone();
                        let msg: Msg = serde_json::from_str(payload)?;
                        let cloned_msg = msg.clone();
                        let to_db = tokio::spawn(async move {
                            if let Err(e) = Self::send_to_db(&mut db_rpc, cloned_msg).await {
                                error!("failed to consume message, error: {:?}", e);
                            }
                        });

                        // send to pusher
                        let mut pusher = self.pusher.clone();
                        let to_pusher = tokio::spawn(async move {
                            if let Err(e) = Self::send_to_pusher(&mut pusher, msg).await {
                                error!("failed to consume message, error: {:?}", e);
                            }
                        });

                        // todo try_join! or join!; we should think about it
                        match tokio::try_join!(to_db, to_pusher) {
                            Ok(_) => {
                                if let Err(e) = self.consumer.commit_message(&m, CommitMode::Async)
                                {
                                    error!("Failed to commit message: {:?}", e);
                                }
                            }
                            Err(err) => {
                                error!("failed to consume message, error: {:?}", err);
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn send_to_db(db_rpc: &mut DbServiceClient<Channel>, msg: Msg) -> Result<(), Error> {
        // don't send it if data type is call xxx
        let mut send_flag = true;
        if msg.data.is_some() {
            // we only skip the single call protocol data for db
            match msg.data.as_ref().unwrap() {
                Data::AgreeSingleCall(_)
                | Data::Candidate(_)
                | Data::SingleCallOffer(_)
                | Data::SingleCallInvite(_)
                | Data::SingleCallInviteAnswer(_) => {
                    send_flag = false;
                }
                _ => {}
            }
        }
        if send_flag {
            db_rpc
                .save_message(SaveMessageRequest {
                    message: Some(MsgToDb::from(msg)),
                })
                .await
                .map_err(|e| Error::InternalServer(e.to_string()))?;
        }

        Ok(())
    }
    pub async fn send_to_pusher(
        pusher: &mut PushServiceClient<Channel>,
        msg: Msg,
    ) -> Result<(), Error> {
        pusher
            .push_msg(SendMsgRequest { message: Some(msg) })
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;
        Ok(())
    }
}
