use crate::message::MsgResponse;
use tonic::Status;

impl From<Status> for MsgResponse {
    fn from(status: Status) -> Self {
        MsgResponse {
            local_id: String::new(),
            server_id: String::new(),
            err: status.message().to_string(),
        }
    }
}
