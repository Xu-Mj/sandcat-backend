use std::sync::Arc;
use std::time::Duration;

use abi::config::Config;
use abi::errors::Error;
use abi::message::chat_service_server::{ChatService, ChatServiceServer};
use abi::message::{MsgResponse, SendMsgRequest};
use axum::async_trait;
use nanoid::nanoid;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use tokio::sync::Mutex;
use tonic::server::NamedService;
use tonic::transport::Server;
use tracing::{error, info};
use utils::typos::{GrpcHealthCheck, Registration};

pub struct ChatRpcService {
    pub kafka: Arc<Mutex<FutureProducer>>,
    pub topic: String,
}

impl ChatRpcService {
    pub fn new(kafka: FutureProducer, topic: String) -> Self {
        Self {
            kafka: Arc::new(Mutex::new(kafka)),
            topic,
        }
    }
    pub async fn start(config: &Config) -> Result<(), Error> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", config.kafka.hosts.join(","))
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Producer creation error");

        let chat_rpc = Self::new(producer, config.kafka.topic.clone());

        // register service
        chat_rpc.register_service(config).await?;
        info!("<chat> rpc service register to service register center");

        // health check
        let (mut reporter, health_service) = tonic_health::server::health_reporter();
        reporter
            .set_serving::<ChatServiceServer<ChatRpcService>>()
            .await;
        info!("<chat> rpc service health check started");

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

    async fn register_service(&self, config: &Config) -> Result<(), Error> {
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
            id: "xmj-chat".to_string(),
            name: config.rpc.chat.name.clone(),
            address: config.rpc.chat.host.clone(),
            port: config.rpc.chat.port,
            tags: config.rpc.chat.tags.clone(),
            check: Some(check),
        };
        center.register(registration).await?;
        Ok(())
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
        let err = match self
            .kafka
            .lock()
            .await
            .send(record, Duration::from_secs(0))
            .await
        {
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
            local_id: msg.send_id,
            server_id: msg.server_id,
            send_time: msg.send_time,
            err,
        }));
    }
}
