use mongodb::bson::Document;
use tonic::Status;

use crate::errors::Error;
use crate::message::msg::Data;
use crate::message::{Msg, MsgResponse, MsgToDb};
use crate::utils;

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
    /// this is for database content
    pub fn content(&self) -> String {
        match self {
            Data::Single(msg) | Data::GroupMsg(msg) => msg.content.clone(),
            Data::SingleCallInviteCancel(_) => String::from("Canceled"),
            Data::SingleCallInviteNotAnswer(_) => String::from("Not Answer"),
            Data::Hangup(msg) => utils::format_milliseconds(msg.sustain),

            // all those can be ignored

            // Data::GroupInvitation(_) => {}
            // Data::GroupMemberExit(_) => {}
            // Data::GroupDismiss(_) => {}
            // Data::GroupDismissOrExitReceived(_) => {}
            // Data::GroupInvitationReceived(_) => {}
            // Data::SingleCallInvite(_) => {}
            // Data::SingleCallInviteAnswer(_) => {}
            // Data::SingleCallOffer(_) => {}
            // Data::AgreeSingleCall(_) => {}
            // Data::Candidate(_) => {}
            // Data::Response(_) => String::new(),
            _ => String::new(),
        }
    }
}

impl TryFrom<Document> for MsgToDb {
    type Error = Error;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        Ok(Self {
            local_id: value.get_str("local_id")?.to_string(),
            server_id: value.get_str("server_id")?.to_string(),
            send_time: value.get_i64("send_time")?,
            content_type: value.get_i32("content_type")?,
            content: value.get_str("content")?.to_string(),
            send_id: value.get_str("send_id")?.to_string(),
            receiver_id: value.get_str("receiver_id")?.to_string(),
        })
    }
}
