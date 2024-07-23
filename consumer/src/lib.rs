use std::sync::Arc;

use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use tracing::{debug, error, warn};

use abi::config::Config;
use abi::errors::Error;
use abi::message::db_service_client::DbServiceClient;
use abi::message::push_service_client::PushServiceClient;
use abi::message::{
    GroupMemSeq, GroupMembersIdRequest, Msg, MsgReadReq, MsgType, SaveGroupMsgRequest,
    SaveMaxSeqRequest, SaveMessageRequest, SendGroupMsgRequest, SendMsgRequest,
};
use cache::Cache;
use utils::service_discovery::LbWithServiceDiscovery;

/// message type: single, group, other
#[derive(Debug, Clone, Eq, PartialEq)]
enum MsgType2 {
    Single,
    Group,
}

pub struct ConsumerService {
    consumer: StreamConsumer,
    /// rpc client
    db_rpc: DbServiceClient<LbWithServiceDiscovery>,
    pusher: PushServiceClient<LbWithServiceDiscovery>,
    cache: Arc<dyn Cache>,
    seq_step: i32,
}

impl ConsumerService {
    pub async fn new(config: &Config) -> Self {
        error!("start kafka consumer{:?}", config.kafka);
        // init kafka consumer
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &config.kafka.group)
            .set("bootstrap.servers", config.kafka.hosts.join(","))
            .set("enable.auto.commit", "false")
            .set(
                "session.timeout.ms",
                config.kafka.consumer.session_timeout.to_string(),
            )
            .set(
                "socket.timeout.ms",
                config.kafka.connect_timeout.to_string(),
            )
            .set("enable.partition.eof", "false")
            .set(
                "auto.offset.reset",
                config.kafka.consumer.auto_offset_reset.clone(),
            )
            .create()
            .expect("Consumer creation failed");

        // todo register to service register center to monitor the service
        // subscribe to topic
        consumer
            .subscribe(&[&config.kafka.topic])
            .expect("Can't subscribe to specified topic");

        // init rpc client
        let pusher = utils::get_rpc_client(config, config.rpc.pusher.name.clone())
            .await
            .unwrap();
        let db_rpc = utils::get_rpc_client(config, config.rpc.db.name.clone())
            .await
            .unwrap();

        let seq_step = config.redis.seq_step;

        let cache = cache::cache(config);

        Self {
            consumer,
            db_rpc,
            pusher,
            cache,
            seq_step,
        }
    }

    pub async fn consume(&mut self) -> Result<(), Error> {
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

    async fn classify_msg_type(&self, mt: MsgType) -> (MsgType2, bool, bool) {
        let msg_type;
        let mut need_increase_seq = false;
        let mut need_history = true;

        match mt {
            MsgType::SingleMsg
            | MsgType::SingleCallInviteNotAnswer
            | MsgType::SingleCallInviteCancel
            | MsgType::Hangup
            | MsgType::ConnectSingleCall
            | MsgType::RejectSingleCall
            | MsgType::FriendApplyReq
            | MsgType::FriendApplyResp
            | MsgType::FriendDelete => {
                // single message and need to increase seq
                msg_type = MsgType2::Single;
                need_increase_seq = true;
            }
            MsgType::GroupMsg => {
                // group message and need to increase seq
                // but not here, need to increase everyone's seq
                msg_type = MsgType2::Group;
            }
            MsgType::GroupInvitation
            | MsgType::GroupInviteNew
            | MsgType::GroupMemberExit
            | MsgType::GroupDismiss
            | MsgType::GroupUpdate => {
                // group message and need to increase seq
                msg_type = MsgType2::Group;
                need_history = false;
            }
            // single call data exchange and don't need to increase seq
            _ => {
                msg_type = MsgType2::Single;
                need_history = false;
            }
        }

        (msg_type, need_increase_seq, need_history)
    }

    /// query members id from cache
    /// if not found, query from db
    async fn get_members_id(&self, group_id: &str) -> Result<Vec<String>, Error> {
        match self.cache.query_group_members_id(group_id).await {
            Ok(list) if !list.is_empty() => Ok(list),
            Ok(_) => {
                warn!("group members id is empty from cache");
                // query from db
                self.query_group_members_id_from_db(group_id).await
            }
            Err(err) => {
                error!("failed to query group members id from cache: {:?}", err);
                Err(err)
            }
        }
    }

    async fn handle_send_seq(&self, user_id: &str) -> Result<(), Error> {
        let mut db_rpc = self.db_rpc.clone();
        let send_seq = self.cache.get_send_seq(user_id).await?;

        if send_seq.0 == send_seq.1 - self.seq_step as i64 {
            db_rpc
                .save_send_max_seq(SaveMaxSeqRequest {
                    user_id: user_id.to_string(),
                })
                .await
                .map_err(|e| Error::InternalServer(e.to_string()))?;
        }
        Ok(())
    }

    async fn increase_message_seq(&self, user_id: &str) -> Result<i64, Error> {
        let mut db_rpc = self.db_rpc.clone();
        let (cur_seq, _, updated) = self.cache.increase_seq(user_id).await?;
        if updated {
            db_rpc
                .save_max_seq(SaveMaxSeqRequest {
                    user_id: user_id.to_string(),
                })
                .await
                .map_err(|e| Error::InternalServer(e.to_string()))?;
        }
        Ok(cur_seq)
    }

    async fn handle_msg_read(&self, msg: Msg) -> Result<(), Error> {
        let data =
            bincode::deserialize(&msg.content).map_err(|e| Error::InternalServer(e.to_string()))?;
        let mut db_rpc = self.db_rpc.clone();

        db_rpc
            .read_msg(MsgReadReq {
                msg_read: Some(data),
            })
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;
        Ok(())
    }

    async fn handle_group_seq(
        &self,
        msg_type: &MsgType2,
        msg: &mut Msg,
    ) -> Result<Vec<GroupMemSeq>, Error> {
        if *msg_type != MsgType2::Group {
            return Ok(vec![]);
        }
        // query group members id from the cache
        let mut members = self.get_members_id(&msg.receiver_id).await?;

        // retain the members id
        members.retain(|id| id != &msg.send_id);

        // increase the members seq
        let seq = self.cache.incr_group_seq(members).await?;

        // we should send the whole list to db module and db module will handle the data

        // judge the message type;
        // we should delete the cache data if the type is group dismiss
        // update the cache if the type is group member exit
        if msg.msg_type == MsgType::GroupDismiss as i32 {
            self.cache.del_group_members(&msg.receiver_id).await?;
        } else if msg.msg_type == MsgType::GroupMemberExit as i32 {
            self.cache
                .remove_group_member_id(&msg.receiver_id, &msg.send_id)
                .await?;
        }

        Ok(seq)
    }

    async fn handle_msg(&self, payload: &str) -> Result<(), Error> {
        debug!("Received message: {:#?}", payload);

        // send to db rpc server
        let mut db_rpc = self.db_rpc.clone();

        let mut msg: Msg = serde_json::from_str(payload)?;

        let mt =
            MsgType::try_from(msg.msg_type).map_err(|e| Error::InternalServer(e.to_string()))?;

        // handle message read type
        if mt == MsgType::Read {
            return self.handle_msg_read(msg).await;
        }

        let (msg_type, need_increase_seq, need_history) = self.classify_msg_type(mt).await;

        // check send seq if need to increase max_seq
        self.handle_send_seq(&msg.send_id).await?;

        // handle receiver seq
        if need_increase_seq {
            let cur_seq = self.increase_message_seq(&msg.receiver_id).await?;
            msg.seq = cur_seq;
        }

        // query members id from cache if the message type is group
        let members = self.handle_group_seq(&msg_type, &mut msg).await?;

        let mut tasks = Vec::with_capacity(2);
        // send to db
        if Self::get_send_to_db_flag(&mt) {
            let cloned_msg = msg.clone();
            let cloned_type = msg_type.clone();
            let cloned_members = members.clone();
            let to_db = tokio::spawn(async move {
                if let Err(e) = Self::send_to_db(
                    &mut db_rpc,
                    cloned_msg,
                    cloned_type,
                    need_history,
                    cloned_members,
                )
                .await
                {
                    error!("failed to send message to db, error: {:?}", e);
                }
            });

            tasks.push(to_db);
        }
        // send to pusher
        let mut pusher = self.pusher.clone();
        let to_pusher = tokio::spawn(async move {
            match msg_type {
                MsgType2::Single => {
                    if let Err(e) = Self::send_single_to_pusher(&mut pusher, msg).await {
                        error!("failed to send message to pusher, error: {:?}", e);
                    }
                }
                MsgType2::Group => {
                    if let Err(e) = Self::send_group_to_pusher(&mut pusher, msg, members).await {
                        error!("failed to send message to pusher, error: {:?}", e);
                    }
                }
            }
        });
        tasks.push(to_pusher);

        futures::future::try_join_all(tasks)
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;

        Ok(())
    }

    /// there is no need to send to db
    /// if the message type is related to call protocol
    #[inline]
    fn get_send_to_db_flag(msg_type: &MsgType) -> bool {
        matches!(
            *msg_type,
            MsgType::ConnectSingleCall
                | MsgType::AgreeSingleCall
                | MsgType::Candidate
                | MsgType::SingleCallOffer
                | MsgType::SingleCallInvite
        )
    }

    async fn send_to_db(
        db_rpc: &mut DbServiceClient<LbWithServiceDiscovery>,
        msg: Msg,
        msg_type: MsgType2,
        need_to_history: bool,
        members: Vec<GroupMemSeq>,
    ) -> Result<(), Error> {
        // match the message type to procedure the different method
        match msg_type {
            MsgType2::Single => {
                let request = SaveMessageRequest::new(msg, need_to_history);
                db_rpc
                    .save_message(request)
                    .await
                    .map_err(|e| Error::InternalServer(e.to_string()))?;
            }
            MsgType2::Group => {
                let request = SaveGroupMsgRequest::new(msg, need_to_history, members);
                db_rpc
                    .save_group_message(request)
                    .await
                    .map_err(|e| Error::InternalServer(e.to_string()))?;
            }
        }

        Ok(())
    }

    async fn send_single_to_pusher(
        pusher: &mut PushServiceClient<LbWithServiceDiscovery>,
        msg: Msg,
    ) -> Result<(), Error> {
        pusher
            .push_single_msg(SendMsgRequest { message: Some(msg) })
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;
        Ok(())
    }

    async fn send_group_to_pusher(
        pusher: &mut PushServiceClient<LbWithServiceDiscovery>,
        msg: Msg,
        members: Vec<GroupMemSeq>,
    ) -> Result<(), Error> {
        pusher
            .push_group_msg(SendGroupMsgRequest {
                message: Some(msg),
                members,
            })
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;
        Ok(())
    }

    /// query members id from database
    /// and set it to cache
    async fn query_group_members_id_from_db(&self, group_id: &str) -> Result<Vec<String>, Error> {
        let request = GroupMembersIdRequest {
            group_id: group_id.to_string(),
        };
        let mut db_rpc = self.db_rpc.clone();
        match db_rpc.group_members_id(request).await {
            Ok(resp) => {
                let members_id = resp.into_inner().members_id;

                // save it to cache
                if let Err(e) = self
                    .cache
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
