use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use nanoid::nanoid;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::error::KafkaError;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use rdkafka::ClientConfig;
use tonic::server::NamedService;
use tonic::transport::Server;
use tracing::{error, info, warn};

use abi::config::Config;
use abi::errors::Error;
use abi::message::chat_service_server::{ChatService, ChatServiceServer};
use abi::message::db_service_client::DbServiceClient;
use abi::message::{GroupMembersIdRequest, MsgResponse, MsgType, SendMsgRequest};
use cache::Cache;
use utils::typos::{GrpcHealthCheck, Registration};
use utils::LbWithServiceDiscovery;

pub struct ChatRpcService {
    kafka: FutureProducer,
    topic: String,
    cache: Arc<dyn Cache>,
    db_rpc: DbServiceClient<LbWithServiceDiscovery>,
}

impl ChatRpcService {
    pub fn new(
        kafka: FutureProducer,
        topic: String,
        cache: Arc<dyn Cache>,
        db_rpc: DbServiceClient<LbWithServiceDiscovery>,
    ) -> Self {
        Self {
            kafka,
            topic,
            cache,
            db_rpc,
        }
    }
    pub async fn start(config: &Config) {
        let broker = config.kafka.hosts.join(",");
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &broker)
            .set(
                "message.timeout.ms",
                config.kafka.producer.timeout.to_string(),
            )
            .set("acks", config.kafka.producer.acks.clone())
            // make sure the message is sent exactly once
            .set("enable.idempotence", "true")
            .set("retries", config.kafka.producer.max_retry.to_string())
            .set(
                "retry.backoff.ms",
                config.kafka.producer.retry_interval.to_string(),
            )
            .create()
            .expect("Producer creation error");

        Self::ensure_topic_exists(&config.kafka.topic, &broker)
            .await
            .expect("Topic creation error");

        // register service
        Self::register_service(config)
            .await
            .expect("Service register error");
        info!("<chat> rpc service register to service register center");

        // health check
        let (mut reporter, health_service) = tonic_health::server::health_reporter();
        reporter
            .set_serving::<ChatServiceServer<ChatRpcService>>()
            .await;
        info!("<chat> rpc service health check started");

        let cache = cache::cache(config);
        let db_rpc = Self::get_db_rpc_client(config)
            .await
            .expect("Db rpc client error");

        let chat_rpc = Self::new(producer, config.kafka.topic.clone(), cache, db_rpc);
        let service = ChatServiceServer::new(chat_rpc);
        info!(
            "<chat> rpc service started at {}",
            config.rpc.chat.rpc_server_url()
        );

        Server::builder()
            .add_service(health_service)
            .add_service(service)
            .serve(config.rpc.chat.rpc_server_url().parse().unwrap())
            .await
            .unwrap();
    }

    async fn get_db_rpc_client(
        config: &Config,
    ) -> Result<DbServiceClient<LbWithServiceDiscovery>, Error> {
        // use service register center to get ws rpc url
        let channel =
            utils::get_channel_with_config(config, &config.rpc.db.name, &config.rpc.db.protocol)
                .await?;
        let db_rpc = DbServiceClient::new(channel);
        Ok(db_rpc)
    }

    async fn register_service(config: &Config) -> Result<(), Error> {
        // register service to service register center
        let center = utils::service_register_center(config);
        let grpc = format!(
            "{}/{}",
            config.rpc.chat.rpc_server_url(),
            <ChatServiceServer<ChatRpcService> as NamedService>::NAME
        );
        let check = GrpcHealthCheck {
            name: config.rpc.chat.name.clone(),
            grpc,
            grpc_use_tls: config.rpc.chat.grpc_health_check.grpc_use_tls,
            interval: format!("{}s", config.rpc.chat.grpc_health_check.interval),
        };
        let registration = Registration {
            id: format!("{}-{}", utils::get_host_name()?, &config.rpc.chat.name),
            name: config.rpc.chat.name.clone(),
            address: config.rpc.chat.host.clone(),
            port: config.rpc.chat.port,
            tags: config.rpc.chat.tags.clone(),
            check: Some(check),
        };
        center.register(registration).await?;
        Ok(())
    }

    async fn ensure_topic_exists(topic_name: &str, brokers: &str) -> Result<(), KafkaError> {
        // Create Kafka AdminClient
        let admin_client: AdminClient<DefaultClientContext> = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .create()?;

        // create topic
        let new_topics = [NewTopic {
            name: topic_name,
            num_partitions: 1,
            replication: TopicReplication::Fixed(1),
            config: vec![],
        }];

        // fixme not find the way to check topic exist
        // so just create it and judge the error,
        // but don't find the error type for topic exist
        // and this way below can work well.
        let options = AdminOptions::new();
        admin_client.create_topics(&new_topics, &options).await?;
        match admin_client.create_topics(&new_topics, &options).await {
            Ok(_) => {
                info!("Topic not exist; create '{}' ", topic_name);
                Ok(())
            }
            Err(KafkaError::AdminOpCreation(_)) => {
                println!("Topic '{}' already exists.", topic_name);
                Ok(())
            }
            Err(err) => Err(err),
        }
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

#[async_trait]
impl ChatService for ChatRpcService {
    /// send message to mq
    /// generate msg id and send time
    async fn send_msg(
        &self,
        request: tonic::Request<SendMsgRequest>,
    ) -> Result<tonic::Response<MsgResponse>, tonic::Status> {
        let mut msg = request
            .into_inner()
            .message
            .ok_or_else(|| tonic::Status::invalid_argument("message is empty"))?;

        // generate msg id
        if !(msg.msg_type == MsgType::GroupDismissOrExitReceived as i32
            || msg.msg_type == MsgType::GroupInvitationReceived as i32
            || msg.msg_type == MsgType::FriendshipReceived as i32)
        {
            msg.server_id = nanoid!();
        }
        msg.send_time = chrono::Local::now()
            .naive_local()
            .and_utc()
            .timestamp_millis();

        let msg_type = MsgType::try_from(msg.msg_type)
            .map_err(|_| tonic::Status::invalid_argument("msg type is invalid"))?;

        // send msg to kafka
        let payload = serde_json::to_string(&msg).unwrap();
        // let kafka generate key, then we need set FutureRecord<String, type>
        let record: FutureRecord<String, String> = FutureRecord::to(&self.topic).payload(&payload);
        self.kafka
            .begin_transaction()
            .map_err(|_| tonic::Status::internal("begin kafka transaction error"))?;

        // increase the user sequence
        match msg_type {
            MsgType::FriendDelete
            | MsgType::FriendApplyReq
            | MsgType::FriendApplyResp
            | MsgType::SingleMsg
            | MsgType::SingleCallInviteNotAnswer
            | MsgType::SingleCallInviteCancel
            | MsgType::Hangup
            | MsgType::ConnectSingleCall
            | MsgType::RejectSingleCall => {
                let seq = self.cache.increase_seq(&msg.receiver_id).await?;
                msg.seq = seq;
            }
            MsgType::GroupMsg
            | MsgType::GroupInvitation
            | MsgType::GroupInviteNew
            | MsgType::GroupMemberExit
            | MsgType::GroupDismiss
            | MsgType::GroupDismissOrExitReceived
            | MsgType::GroupInvitationReceived
            | MsgType::GroupUpdate => {
                // query group members id from the cache
                let mut members = self.get_members_id(&msg.receiver_id).await?;

                // retain the members id
                members.retain(|id| id != &msg.send_id);

                // increase the members seq
                self.cache.incr_group_seq(&members).await?;
            }
            _ => {}
        }

        let err = match self.kafka.send(record, Duration::from_secs(0)).await {
            Ok(_) => String::new(),
            Err((err, msg)) => {
                error!(
                    "send msg to kafka error: {:?}; owned message: {:?}",
                    err, msg
                );
                err.to_string()
            }
        };

        return Ok(tonic::Response::new(MsgResponse {
            local_id: msg.local_id,
            server_id: msg.server_id,
            send_time: msg.send_time,
            err,
        }));
    }
}
