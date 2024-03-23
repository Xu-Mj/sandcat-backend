use crate::message::MsgResponse;
use tonic::Status;

impl From<Status> for MsgResponse {
    fn from(status: Status) -> Self {
        MsgResponse {
            local_id: String::new(),
            server_id: String::new(),
            send_time: 0,
            err: status.message().to_string(),
        }
    }
}
