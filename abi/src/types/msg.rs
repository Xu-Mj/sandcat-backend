use tonic::Status;

use crate::message::group_msg_wrapper::GroupMsg;
use crate::message::msg::Data;
use crate::message::{Msg, MsgResponse, MsgToDb};

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

impl From<Msg> for MsgToDb {
    fn from(value: Msg) -> Self {
        let content = if value.data.is_none() {
            String::new()
        } else {
            value.data.unwrap().content()
        };
        Self {
            send_id: value.send_id,
            receiver_id: value.receiver_id,
            local_id: value.local_id,
            server_id: value.server_id,
            send_time: value.send_time,
            content_type: 0,
            content,
        }
    }
}

impl Data {
    pub fn content(&self) -> String {
        match self {
            Data::Single(msg) => msg.content.clone(),
            Data::Group(msg) => {
                if msg.group_msg.is_none() {
                    String::new()
                } else {
                    match msg.group_msg.as_ref().unwrap() {
                        GroupMsg::Invitation(_) => String::from("Group Invitation"),
                        GroupMsg::Message(msg) => msg.content.clone(),
                        GroupMsg::MemberExit(_) => String::from("Group Member Exit"),
                        GroupMsg::Dismiss(_) => String::from("Group Dismiss"),
                        _ => String::new(),
                    }
                }
            }
            Data::Response(_) => String::new(),
        }
    }
}
