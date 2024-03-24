use std::sync::Arc;
use std::time::Duration;

use abi::message::chat_service_server::ChatService;
use abi::message::{MsgResponse, SendMsgRequest};
use axum::async_trait;
use nanoid::nanoid;
use rdkafka::producer::{FutureProducer, FutureRecord};
use tokio::sync::Mutex;
use tracing::error;

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
