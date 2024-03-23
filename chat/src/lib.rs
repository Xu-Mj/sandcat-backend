use std::sync::Arc;

use abi::message::chat_service_server::ChatService;
use abi::message::{MsgResponse, SendMsgRequest};
use axum::async_trait;
use kafka::producer::{Producer, Record};
use nanoid::nanoid;
use tokio::sync::Mutex;
use tracing::error;

pub struct ChatRpcService {
    pub kafka: Arc<Mutex<Producer>>,
}

impl ChatRpcService {
    pub fn new(kafka: Producer) -> Self {
        Self {
            kafka: Arc::new(Mutex::new(kafka)),
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
        let record = Record::from_value("test", serde_json::to_string(&msg).unwrap());

        let err = match self.kafka.lock().await.send(&record) {
            Ok(_) => String::new(),
            Err(err) => {
                error!("send msg to kafka error: {:?}", err);
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
