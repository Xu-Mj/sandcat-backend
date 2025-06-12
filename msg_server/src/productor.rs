use std::time::Duration;

use async_trait::async_trait;
use nanoid::nanoid;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::error::KafkaError;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use synapse::health::{HealthServer, HealthService};
use tonic::transport::Server;
use tracing::{error, info};

use abi::config::{Component, Config};
use abi::message::chat_service_server::{ChatService, ChatServiceServer};
use abi::message::{MsgResponse, MsgType, SendMsgRequest};

pub struct ChatRpcService {
    kafka: FutureProducer,
    topic: String,
}

impl ChatRpcService {
    pub fn new(kafka: FutureProducer, topic: String) -> Self {
        Self { kafka, topic }
    }
    pub async fn start(config: &Config) {
        let broker = config.kafka.hosts.join(",");
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &broker)
            .set(
                "message.timeout.ms",
                config.kafka.producer.timeout.to_string(),
            )
            .set(
                "socket.timeout.ms",
                config.kafka.connect_timeout.to_string(),
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

        Self::ensure_topic_exists(&config.kafka.topic, &broker, config.kafka.connect_timeout)
            .await
            .expect("Topic creation error");

        // register service
        utils::register_service(config, Component::MessageServer)
            .await
            .expect("Service register error");
        info!("<chat> rpc service register to service register center");

        // health check
        let health_service = HealthServer::new(HealthService::new());
        info!("<chat> rpc service health check started");

        let chat_rpc = Self::new(producer, config.kafka.topic.clone());
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

    async fn ensure_topic_exists(
        topic_name: &str,
        brokers: &str,
        timeout: u16,
    ) -> Result<(), KafkaError> {
        // Create Kafka AdminClient
        let admin_client: AdminClient<DefaultClientContext> = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("socket.timeout.ms", timeout.to_string())
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
            .ok_or(tonic::Status::invalid_argument("message is empty"))?;

        // generate msg id
        if !(msg.msg_type == MsgType::GroupDismissOrExitReceived as i32
            || msg.msg_type == MsgType::GroupInvitationReceived as i32
            || msg.msg_type == MsgType::FriendshipReceived as i32)
        {
            msg.server_id = nanoid!();
        }
        msg.send_time = chrono::Utc::now().timestamp_millis();

        // send msg to kafka
        let payload = serde_json::to_string(&msg).unwrap();
        // let kafka generate key, then we need set FutureRecord<String, type>
        let record: FutureRecord<String, String> = FutureRecord::to(&self.topic).payload(&payload);

        info!("send msg to kafka: {:?}", record);
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
            client_id: msg.client_id,
            server_id: msg.server_id,
            send_time: msg.send_time,
            err,
        }));
    }
}
