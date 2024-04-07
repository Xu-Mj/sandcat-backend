use std::sync::Arc;

use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use tonic::transport::Channel;
use tracing::{debug, error, warn};

use abi::config::Config;
use abi::errors::Error;
use abi::message::db_service_client::DbServiceClient;
use abi::message::push_service_client::PushServiceClient;
use abi::message::{
    GroupMembersIdRequest, Msg, MsgType, SaveMessageRequest, SendGroupMsgRequest, SendMsgRequest,
};
use cache::Cache;

/// message type: single, group, other
#[derive(Debug, Clone)]
enum MsgType2 {
    Single,
    Group,
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

    async fn classify_msg_type(&self, mt: MsgType) -> (MsgType2, bool) {
        let msg_type;
        let mut need_increase_seq = true;

        match mt {
            MsgType::SingleMsg
            | MsgType::SingleCallInviteNotAnswer
            | MsgType::SingleCallInviteCancel
            | MsgType::Hangup
            | MsgType::AgreeSingleCall => {
                // single message and need to increase seq
                msg_type = MsgType2::Single;
            }
            MsgType::GroupInvitation
            | MsgType::GroupMemberExit
            | MsgType::GroupDismiss
            | MsgType::GroupUpdate => {
                // group message and need to increase seq
                msg_type = MsgType2::Group;
            }

            // single call data exchange and don't need to increase seq
            _ => {
                msg_type = MsgType2::Single;
                need_increase_seq = false;
            }
        }

        (msg_type, need_increase_seq)
    }

    /// all single and group message need to increase seq,
    /// because we use the same message receive box
    async fn increase_seq(&self, user_id: &str) -> Result<i64, Error> {
        match self.cache.get_seq(user_id).await {
            Ok(seq) => Ok(seq),
            Err(err) => {
                error!("failed to get seq, error: {:?}", err);
                Err(err)
            }
        }
    }

    /// todo we need to handle the errors, like: should we use something like transaction?
    async fn handle_msg(&self, payload: &str) -> Result<(), Error> {
        debug!("Received message: {:#?}", payload);

        // send to db rpc server
        let mut db_rpc = self.db_rpc.clone();
        let mut msg: Msg = serde_json::from_str(payload)?;
        let msg_type =
            MsgType::try_from(msg.msg_type).map_err(|e| Error::InternalServer(e.to_string()))?;
        let (msg_type, need_increase_seq) = self.classify_msg_type(msg_type).await;
        if need_increase_seq {
            msg.seq = self.increase_seq(&msg.receiver_id).await?;
        }

        // send to db
        let cloned_msg = msg.clone();
        let cloned_type = msg_type.clone();
        let to_db = tokio::spawn(async move {
            if let Err(e) = Self::send_to_db(&mut db_rpc, cloned_msg, cloned_type).await {
                error!("failed to send message to db, error: {:?}", e);
            }
        });

        // send to pusher
        let mut pusher = self.pusher.clone();
        let cache = self.cache.clone();
        let mut db = self.db_rpc.clone();
        let to_pusher = tokio::spawn(async move {
            match msg_type {
                MsgType2::Single => {
                    if let Err(e) = Self::send_single_to_pusher(&mut pusher, msg).await {
                        error!("failed to send message to pusher, error: {:?}", e);
                    }
                }
                MsgType2::Group => {
                    // query group members id from the cache
                    let members = match cache.query_group_members_id(&msg.receiver_id).await {
                        Ok(list) if !list.is_empty() => list,
                        Ok(_) => {
                            warn!("group members id is empty from cache");
                            // query from db
                            match Self::query_group_members_id_from_db(
                                &mut db,
                                cache,
                                &msg.receiver_id,
                            )
                            .await
                            {
                                Ok(list) => list,
                                Err(_) => return,
                            }
                        }
                        Err(err) => {
                            error!("failed to query group members id from cache: {:?}", err);
                            return;
                        }
                    };
                    if let Err(e) = Self::send_group_to_pusher(&mut pusher, msg, members).await {
                        error!("failed to send message to pusher, error: {:?}", e);
                    }
                }
            }
        });

        // todo try_join! or join!; we should think about it
        if let Err(err) = tokio::try_join!(to_db, to_pusher) {
            error!("failed to consume message, error: {:?}", err);
            return Err(Error::InternalServer(err.to_string()));
        }
        Ok(())
    }

    async fn send_to_db(
        db_rpc: &mut DbServiceClient<Channel>,
        msg: Msg,
        msg_type: MsgType2,
    ) -> Result<(), Error> {
        // don't send it if data type is call xxx
        let mut send_flag = true;
        let msg_type2 =
            MsgType::try_from(msg.msg_type).map_err(|e| Error::InternalServer(e.to_string()))?;
        // only skip the single call protocol data for db
        match msg_type2 {
            MsgType::AgreeSingleCall
            | MsgType::Candidate
            | MsgType::SingleCallOffer
            | MsgType::SingleCallInvite
            | MsgType::SingleCallInviteAnswer => {
                send_flag = false;
            }
            _ => {}
        }

        if !send_flag {
            return Ok(());
        }

        // match the message type to procedure the different method
        let request = SaveMessageRequest { message: Some(msg) };

        match msg_type {
            MsgType2::Single => {
                db_rpc
                    .save_message(request)
                    .await
                    .map_err(|e| Error::InternalServer(e.to_string()))?;
            }
            MsgType2::Group => {
                db_rpc
                    .save_group_message(request)
                    .await
                    .map_err(|e| Error::InternalServer(e.to_string()))?;
            }
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

    async fn query_group_members_id_from_db(
        db_rpc: &mut DbServiceClient<Channel>,
        cache: Arc<Box<dyn Cache>>,
        group_id: &str,
    ) -> Result<Vec<String>, Error> {
        let request = GroupMembersIdRequest {
            group_id: group_id.to_string(),
        };
        match db_rpc.group_members_id(request).await {
            Ok(resp) => {
                let members_id = resp.into_inner().members_id;

                // save it to cache
                if let Err(e) = cache
                    .save_group_members_id(group_id, members_id.clone())
                    .await
                {
                    error!("failed to save group members id to cache: {:?}", e);
                }

                Ok(members_id)
            }
            Err(e) => {
                error!("failed to query group members id from db: {:?}", e);
                Err(Error::InternalServer(e.to_string()))
            }
        }
    }
}
