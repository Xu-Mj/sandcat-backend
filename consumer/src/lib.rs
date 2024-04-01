use abi::config::Config;
use abi::errors::Error;
use abi::message::db_service_client::DbServiceClient;
use abi::message::msg::Data;
use abi::message::push_service_client::PushServiceClient;
use abi::message::{Msg, MsgToDb, SaveMessageRequest, SendGroupMsgRequest, SendMsgRequest};
use cache::Cache;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use std::sync::Arc;
use tonic::transport::Channel;
use tracing::{debug, error, warn};
/// message type: single, group, other
#[derive(Debug, Clone)]
enum MsgType {
    Single,
    Group,
    Other,
}

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
                        if let Err(e) = self.handle_msg(payload).await {
                            error!("Failed to handle message: {:?}", e);
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

    /// todo we need to handle the errors, like: should we use something like transaction?
    async fn handle_msg(&self, payload: &str) -> Result<(), Error> {
        debug!("Received message: {:#?}", payload);

        // send to db rpc server
        let mut db_rpc = self.db_rpc.clone();
        let mut msg: Msg = serde_json::from_str(payload)?;

        let mut msg_type = MsgType::Other;
        let mut need_increase_seq = false;
        // increase seq
        if msg.data.is_some() {
            match msg.data.as_ref().unwrap() {
                Data::Single(_)
                | Data::SingleCallInviteNotAnswer(_)
                | Data::SingleCallInviteCancel(_)
                | Data::Hangup(_)
                | Data::AgreeSingleCall(_) => {
                    // single message and need to increase seq
                    msg_type = MsgType::Single;
                    need_increase_seq = true;
                }
                Data::GroupInvitation(_)
                | Data::GroupMemberExit(_)
                | Data::GroupDismiss(_)
                | Data::GroupUpdate(_) => {
                    // group message and need to increase seq
                    msg_type = MsgType::Group;
                    need_increase_seq = true;
                }

                // single call data exchange and don't need to increase seq
                _ => {
                    msg_type = MsgType::Single;
                    need_increase_seq = false;
                }
            }
        }

        if need_increase_seq {
            match self.cache.get_seq(&msg.receiver_id).await {
                Ok(seq) => {
                    msg.seq = seq;
                }
                Err(err) => {
                    error!("failed to get seq, error: {:?}", err);
                    return Err(err);
                }
            }
        }

        // send to db
        let cloned_msg = msg.clone();
        let cloned_type = msg_type.clone();
        let to_db = tokio::spawn(async move {
            match cloned_type {
                MsgType::Single => {
                    if let Err(e) = Self::send_to_db(&mut db_rpc, cloned_msg).await {
                        error!("failed to send message to db, error: {:?}", e);
                    }
                }
                MsgType::Group => {
                    // todo send to db by group method
                    if let Err(e) = Self::send_to_db(&mut db_rpc, cloned_msg).await {
                        error!("failed to send message to db, error: {:?}", e);
                    }
                }
                MsgType::Other => {}
            }
        });

        // send to pusher
        let mut pusher = self.pusher.clone();
        let cache = self.cache.clone();
        // let db = self.db_rpc.clone();
        let to_pusher = tokio::spawn(async move {
            match msg_type {
                MsgType::Single => {
                    if let Err(e) = Self::send_single_to_pusher(&mut pusher, msg).await {
                        error!("failed to send message to pusher, error: {:?}", e);
                    }
                }
                MsgType::Group => {
                    // query group members id from the cache
                    let members = match cache.query_group_members_id(&msg.receiver_id).await {
                        Ok(list) => {
                            if list.is_empty() {
                                warn!("group members id is empty from cache");
                                // todo query from db

                                return;
                            } else {
                                list
                            }
                        }
                        Err(err) => {
                            error!(
                                "failed to query group members id from cache, error: {:?}",
                                err
                            );
                            return;
                        }
                    };
                    if let Err(e) = Self::send_group_to_pusher(&mut pusher, msg, members).await {
                        error!("failed to send message to pusher, error: {:?}", e);
                    }
                }
                MsgType::Other => {}
            }
        });

        // todo try_join! or join!; we should think about it
        if let Err(err) = tokio::try_join!(to_db, to_pusher) {
            error!("failed to consume message, error: {:?}", err);
            return Err(Error::InternalServer(err.to_string()));
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
            // todo we should match the message type to procedure the different method
            db_rpc
                .save_message(SaveMessageRequest {
                    message: Some(MsgToDb::from(msg)),
                })
                .await
                .map_err(|e| Error::InternalServer(e.to_string()))?;
        }

        Ok(())
    }
    async fn send_single_to_pusher(
        pusher: &mut PushServiceClient<Channel>,
        msg: Msg,
    ) -> Result<(), Error> {
        pusher
            .push_single_msg(SendMsgRequest { message: Some(msg) })
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;
        Ok(())
    }

    async fn send_group_to_pusher(
        pusher: &mut PushServiceClient<Channel>,
        msg: Msg,
        members_id: Vec<String>,
    ) -> Result<(), Error> {
        pusher
            .push_group_msg(SendGroupMsgRequest {
                message: Some(msg),
                members_id,
            })
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;
        Ok(())
    }
}
