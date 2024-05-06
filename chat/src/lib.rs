use std::time::Duration;

use async_trait::async_trait;
use nanoid::nanoid;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::error::KafkaError;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use tonic::server::NamedService;
use tonic::transport::Server;
use tracing::{error, info};

use abi::config::Config;
use abi::errors::Error;
use abi::message::chat_service_server::{ChatService, ChatServiceServer};
use abi::message::{MsgResponse, SendMsgRequest};
use utils::typos::{GrpcHealthCheck, Registration};

pub struct ChatRpcService {
    pub kafka: FutureProducer,
    pub topic: String,
}

impl ChatRpcService {
    pub fn new(kafka: FutureProducer, topic: String) -> Self {
        Self { kafka, topic }
    }
    pub async fn start(config: &Config) -> Result<(), Error> {
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
        Self::register_service(config).await?;
        info!("<chat> rpc service register to service register center");

        // health check
        let (mut reporter, health_service) = tonic_health::server::health_reporter();
        reporter
            .set_serving::<ChatServiceServer<ChatRpcService>>()
            .await;
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
        Ok(())
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
}

#[async_trait]
impl ChatService for ChatRpcService {
    /// send message to mq
    /// generate msg id and send time
    async fn send_msg(
        &self,
        request: tonic::Request<SendMsgRequest>,
    ) -> Result<tonic::Response<MsgResponse>, tonic::Status> {
        let inner = request.into_inner().message;
        if inner.is_none() {
            return Err(tonic::Status::invalid_argument("message is empty"));
        }

        let mut msg = inner.unwrap();

        // generate msg id
        msg.server_id = nanoid!();
        msg.send_time = chrono::Local::now()
            .naive_local()
            .and_utc()
            .timestamp_millis();

        // send msg to kafka
        let payload = serde_json::to_string(&msg).unwrap();
        // let kafka generate key, then we need set FutureRecord<String, type>
        let record: FutureRecord<String, String> = FutureRecord::to(&self.topic).payload(&payload);
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
