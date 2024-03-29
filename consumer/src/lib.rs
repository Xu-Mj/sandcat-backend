use abi::config::Config;
use abi::errors::Error;
use abi::message::db_service_client::DbServiceClient;
use abi::message::msg::Data;
use abi::message::push_service_client::PushServiceClient;
use abi::message::{Msg, MsgToDb, SaveMessageRequest, SendMsgRequest};
use cache::Cache;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use std::sync::Arc;
use tonic::transport::Channel;
use tracing::{debug, error};

pub struct ConsumerService {
    consumer: StreamConsumer,
    /// rpc client
    db_rpc: DbServiceClient<Channel>,
    pusher: PushServiceClient<Channel>,
    cache: Arc<Box<dyn Cache>>,
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
        let db_rpc = Self::get_db_rpc_client(config).await.unwrap();

        let pusher = Self::get_pusher_rpc_client(config).await.unwrap();

        let cache = cache::cache(config);

        Self {
            consumer,
            db_rpc,
            pusher,
            cache: Arc::new(cache),
        }
    }

    async fn get_db_rpc_client(config: &Config) -> Result<DbServiceClient<Channel>, Error> {
        // use service register center to get ws rpc url
        let channel =
            utils::get_rpc_channel_by_name(config, &config.rpc.db.name, &config.rpc.db.protocol)
                .await?;
        let db_rpc = DbServiceClient::new(channel);
        Ok(db_rpc)
    }

    async fn get_pusher_rpc_client(config: &Config) -> Result<PushServiceClient<Channel>, Error> {
        let channel = utils::get_rpc_channel_by_name(
            config,
            &config.rpc.pusher.name,
            &config.rpc.pusher.protocol,
        )
        .await?;
        let push_rpc = PushServiceClient::new(channel);
        Ok(push_rpc)
    }

    pub async fn consume(&mut self) -> Result<(), Error> {
        // 开始消费消息
        loop {
            match self.consumer.recv().await {
                Err(e) => error!("Kafka error: {}", e),
                Ok(m) => {
                    if let Some(Ok(payload)) = m.payload_view::<str>() {
                        self.handle_msg(payload).await?;
                        if let Err(e) = self.consumer.commit_message(&m, CommitMode::Async) {
                            error!("Failed to commit message: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    /// todo we need to handle the errors, like: should we use something like transaction?
    async fn handle_msg(&self, payload: &str) -> Result<(), Error> {
        debug!("Received message: {:#?}", payload);

        // send to db rpc server
        let mut db_rpc = self.db_rpc.clone();
        let mut msg: Msg = serde_json::from_str(payload)?;

        // increase seq
        match self.cache.get_seq(&msg.receiver_id).await {
            Ok(seq) => {
                msg.seq = seq;
            }
            Err(err) => {
                error!("failed to get seq, error: {:?}", err);
            }
        }

        // send to db
        let cloned_msg = msg.clone();
        let to_db = tokio::spawn(async move {
            if let Err(e) = Self::send_to_db(&mut db_rpc, cloned_msg).await {
                error!("failed to send message to db, error: {:?}", e);
            }
        });

        // send to pusher
        let mut pusher = self.pusher.clone();
        let to_pusher = tokio::spawn(async move {
            if let Err(e) = Self::send_to_pusher(&mut pusher, msg).await {
                error!("failed to send message to pusher, error: {:?}", e);
            }
        });

        // todo try_join! or join!; we should think about it
        if let Err(err) = tokio::try_join!(to_db, to_pusher) {
            error!("failed to consume message, error: {:?}", err);
        }
        Ok(())
    }

    async fn send_to_db(db_rpc: &mut DbServiceClient<Channel>, msg: Msg) -> Result<(), Error> {
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
    async fn send_to_pusher(
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
