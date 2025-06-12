use std::sync::Arc;

use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use tracing::{debug, error, info, warn};

use abi::config::Config;
use abi::errors::Error;
use abi::message::{GroupMemSeq, Msg, MsgRead, MsgType};
use cache::Cache;
use db::message::MsgRecBoxRepo;
use db::{msg_rec_box_repo, DbRepo};

use crate::pusher::{push_service, Pusher};

/// message type: single, group, other
#[derive(Debug, Clone, Eq, PartialEq)]
enum MsgType2 {
    Single,
    Group,
}

pub struct ConsumerService {
    consumer: StreamConsumer,
    db: Arc<DbRepo>,
    msg_box: Arc<dyn MsgRecBoxRepo>,
    pusher: Arc<dyn Pusher>,
    cache: Arc<dyn Cache>,
    seq_step: i32,
}

impl ConsumerService {
    pub async fn new(config: &Config) -> Self {
        info!("start kafka consumer:\t{:?}", config.kafka);
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

        let pusher = push_service(config).await;
        let db = Arc::new(DbRepo::new(config).await);

        let seq_step = config.redis.seq_step;

        let cache = cache::cache(config);
        let msg_box = msg_rec_box_repo(config).await;

        Self {
            consumer,
            db,
            msg_box,
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

    // todo handle error for the task
    async fn handle_msg(&self, payload: &str) -> Result<(), Error> {
        debug!("Received message: {:#?}", payload);

        let mut msg: Msg = serde_json::from_str(payload)?;

        let mt = MsgType::try_from(msg.msg_type).map_err(Error::internal)?;

        // handle message read type
        if mt == MsgType::Read {
            return self.handle_msg_read(msg).await;
        }

        let (msg_type, need_increase_seq, need_history) = self.classify_msg_type(mt).await;

        // check send seq if need to increase max_seq
        self.handle_send_seq(&msg.sender_id).await?;

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
            // send to db rpc server
            let db = self.db.clone();
            let msg_box = self.msg_box.clone();
            let to_db = tokio::spawn(async move {
                if let Err(e) = Self::send_to_db(
                    db,
                    msg_box,
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
        let pusher = self.pusher.clone();
        let to_pusher = tokio::spawn(async move {
            match msg_type {
                MsgType2::Single => {
                    if let Err(e) = pusher.push_single_msg(msg).await {
                        error!("failed to send message to pusher, error: {:?}", e);
                    }
                }
                MsgType2::Group => {
                    if let Err(e) = pusher.push_group_msg(msg, members).await {
                        error!("failed to send message to pusher, error: {:?}", e);
                    }
                }
            }
        });
        tasks.push(to_pusher);

        futures::future::try_join_all(tasks)
            .await
            .map_err(Error::internal)?;

        Ok(())
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
            MsgType::GroupMsg
            | MsgType::GroupFile
            | MsgType::GroupPoll
            | MsgType::GroupAnnouncement => {
                // 这些群组消息需要增加序列号并保存历史记录
                // group message and need to increase seq
                // but not here, need to increase everyone's seq
                msg_type = MsgType2::Group;
            }
            MsgType::GroupInvitation
            | MsgType::GroupInviteNew
            | MsgType::GroupMemberExit
            | MsgType::GroupRemoveMember
            | MsgType::GroupDismiss
            | MsgType::GroupUpdate
            | MsgType::GroupMute => {
                // 群组操作类消息，不需要保存历史记录
                // group message and need to increase seq
                msg_type = MsgType2::Group;
                need_history = false;
            }
            // single call data exchange and don't need to increase seq
            MsgType::GroupDismissOrExitReceived
            | MsgType::GroupInvitationReceived
            | MsgType::FriendBlack
            | MsgType::SingleCallInvite
            | MsgType::AgreeSingleCall
            | MsgType::SingleCallOffer
            | MsgType::Candidate
            | MsgType::Read
            | MsgType::MsgRecResp
            | MsgType::Notification
            | MsgType::Service
            | MsgType::FriendshipReceived => {
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
        let send_seq = self.cache.get_send_seq(user_id).await?;

        if send_seq.0 == send_seq.1 - self.seq_step as i64 {
            self.db.seq.save_max_seq(user_id).await?;
        }
        Ok(())
    }

    async fn increase_message_seq(&self, user_id: &str) -> Result<i64, Error> {
        let (cur_seq, _, updated) = self.cache.increase_seq(user_id).await?;
        if updated {
            self.db.seq.save_max_seq(user_id).await?;
        }
        Ok(cur_seq)
    }

    async fn handle_msg_read(&self, msg: Msg) -> Result<(), Error> {
        let data: MsgRead = bincode::deserialize(&msg.content)?;

        self.msg_box.msg_read(&data.user_id, &data.msg_seq).await?;
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
        members.retain(|id| id != &msg.sender_id);

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
                .remove_group_member_id(&msg.receiver_id, &msg.sender_id)
                .await?;
        } else if msg.msg_type == MsgType::GroupRemoveMember as i32 {
            let data: Vec<String> = bincode::deserialize(&msg.content)?;

            let member_ids_ref: Vec<&str> = data.iter().map(AsRef::as_ref).collect();
            self.cache
                .remove_group_member_batch(&msg.group_id, &member_ids_ref)
                .await?;
        }

        Ok(seq)
    }

    /// there is no need to send to db
    /// if the message type is related to call protocol
    #[inline]
    fn get_send_to_db_flag(msg_type: &MsgType) -> bool {
        !matches!(
            *msg_type,
            MsgType::ConnectSingleCall
                | MsgType::AgreeSingleCall
                | MsgType::Candidate
                | MsgType::SingleCallOffer
                | MsgType::SingleCallInvite
        )
    }

    async fn send_to_db(
        db: Arc<DbRepo>,
        msg_box: Arc<dyn MsgRecBoxRepo>,
        msg: Msg,
        msg_type: MsgType2,
        need_to_history: bool,
        members: Vec<GroupMemSeq>,
    ) -> Result<(), Error> {
        // match the message type to procedure the different method
        match msg_type {
            MsgType2::Single => {
                Self::handle_message(db, msg_box, msg, need_to_history).await?;
            }
            MsgType2::Group => {
                Self::handle_group_message(db, msg_box, msg, need_to_history, members).await?;
            }
        }

        Ok(())
    }

    /// query members id from database
    /// and set it to cache
    async fn query_group_members_id_from_db(&self, group_id: &str) -> Result<Vec<String>, Error> {
        let members_id = self.db.group.query_group_members_id(group_id).await?;

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

    async fn handle_message(
        db: Arc<DbRepo>,
        msg_box: Arc<dyn MsgRecBoxRepo>,
        message: Msg,
        need_to_history: bool,
    ) -> Result<(), Error> {
        // task 1 save message to postgres

        let mut tasks = Vec::with_capacity(2);
        if !need_to_history {
            let cloned_msg = message.clone();
            let db_task = tokio::spawn(async move {
                if let Err(e) = db.msg.save_message(cloned_msg).await {
                    tracing::error!("save message to db failed: {}", e);
                }
            });
            tasks.push(db_task);
        }

        // task 2 save message to mongodb
        let msg_rec_box_task = tokio::spawn(async move {
            // if the message type is friendship/group-operation delivery, we should delete it from mongodb
            if message.msg_type == MsgType::GroupDismissOrExitReceived as i32
                || message.msg_type == MsgType::GroupInvitationReceived as i32
                || message.msg_type == MsgType::FriendshipReceived as i32
            {
                if let Err(e) = msg_box.delete_message(&message.server_id).await {
                    tracing::error!("delete message from mongodb failed: {}", e);
                }
                return;
            }
            if let Err(e) = msg_box.save_message(&message).await {
                tracing::error!("save message to mongodb failed: {}", e);
            }
        });
        tasks.push(msg_rec_box_task);

        // wait all tasks
        futures::future::try_join_all(tasks)
            .await
            .map_err(Error::internal)?;
        Ok(())
    }

    async fn handle_group_message(
        db: Arc<DbRepo>,
        msg_box: Arc<dyn MsgRecBoxRepo>,
        message: Msg,
        need_to_history: bool,
        members: Vec<GroupMemSeq>,
    ) -> Result<(), Error> {
        // task 1 save message to postgres
        // update the user's seq in postgres
        let need_update = members
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                if item.need_update {
                    members.get(index).map(|v| v.mem_id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();

        let cloned_msg = if need_to_history {
            Some(message.clone())
        } else {
            None
        };

        let db_task = tokio::spawn(async move {
            if !need_update.is_empty() {
                if let Err(err) = db.seq.save_max_seq_batch(&need_update).await {
                    tracing::error!("save max seq batch failed: {}", err);
                    return Err(err);
                };
            }

            if let Some(cloned_msg) = cloned_msg {
                if let Err(e) = db.msg.save_message(cloned_msg).await {
                    tracing::error!("save message to db failed: {}", e);
                    return Err(e);
                }
            }
            Ok(())
        });

        // task 2 save message to mongodb
        let msg_rec_box_task = tokio::spawn(async move {
            if let Err(e) = msg_box.save_group_msg(message, members).await {
                tracing::error!("save message to mongodb failed: {}", e);
                return Err(e);
            }
            Ok(())
        });

        // wait all tasks complete
        let (db_result, msg_rec_box_result) =
            tokio::try_join!(db_task, msg_rec_box_task).map_err(Error::internal)?;

        db_result?;
        msg_rec_box_result?;

        Ok(())
    }
}
