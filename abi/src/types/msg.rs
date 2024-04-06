use mongodb::bson::Document;
use tonic::Status;

use crate::errors::Error;
use crate::message::{Msg, MsgResponse, MsgType, SendMsgRequest, UserAndGroupId};

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

/// maybe there is the performance issue
impl TryFrom<Document> for Msg {
    type Error = Error;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        Ok(Self {
            local_id: value.get_str("local_id")?.to_string(),
            server_id: value.get_str("server_id")?.to_string(),
            create_time: 0,
            send_time: value.get_i64("send_time")?,
            content_type: value.get_i32("content_type")?,
            content: value.get_binary_generic("content")?.to_vec(),
            send_id: value.get_str("send_id")?.to_string(),
            receiver_id: value.get_str("receiver_id")?.to_string(),
            seq: value.get_i64("seq")?,
            group_id: value.get_str("group_id")?.to_string(),
            msg_type: value.get_i32("msg_type")?,
            is_read: value.get_bool("is_read")?,
            // those do not save to mongodb
            sdp: None,
            sdp_mid: None,
            sdp_m_index: None,
            call_agree: false,
        })
    }
}

impl SendMsgRequest {
    pub fn new_with_friend_ship_req(send_id: String, receiver_id: String, fs: Vec<u8>) -> Self {
        Self {
            message: Some(Msg {
                send_id,
                receiver_id,
                send_time: chrono::Local::now().timestamp_millis(),
                content: fs,
                msg_type: MsgType::FriendApplyReq as i32,
                ..Default::default()
            }),
        }
    }

    pub fn new_with_friend_ship_resp(receiver_id: String, fs: Vec<u8>) -> Self {
        Self {
            message: Some(Msg {
                receiver_id,
                content: fs,
                msg_type: MsgType::FriendApplyResp as i32,
                send_time: chrono::Local::now().timestamp_millis(),
                ..Default::default()
            }),
        }
    }

    /// when dismiss group, send id is the owner id,
    /// when member exit group, send id is the member id
    pub fn new_with_group_operation(
        send_id: String,
        receiver_id: String,
        msg_type: MsgType,
    ) -> Self {
        Self {
            message: Some(Msg {
                send_id,
                receiver_id,
                send_time: chrono::Local::now().timestamp_millis(),
                msg_type: msg_type as i32,
                ..Default::default()
            }),
        }
    }

    pub fn new_with_group_invitation(
        send_id: String,
        receiver_id: String,
        invitation: Vec<u8>,
    ) -> Self {
        Self {
            message: Some(Msg {
                send_id,
                receiver_id,
                send_time: chrono::Local::now().timestamp_millis(),
                msg_type: MsgType::GroupInvitation as i32,
                content: invitation,
                ..Default::default()
            }),
        }
    }

    pub fn new_with_group_invite_new(
        send_id: String,
        receiver_id: String,
        invitation: Vec<u8>,
    ) -> Self {
        Self {
            message: Some(Msg {
                send_id,
                receiver_id,
                send_time: chrono::Local::now().timestamp_millis(),
                msg_type: MsgType::GroupInviteNew as i32,
                content: invitation,
                ..Default::default()
            }),
        }
    }

    pub fn new_with_group_update(send_id: String, receiver_id: String, msg: Vec<u8>) -> Self {
        Self {
            message: Some(Msg {
                send_id,
                receiver_id,
                send_time: chrono::Local::now().timestamp_millis(),
                msg_type: MsgType::GroupUpdate as i32,
                content: msg,
                ..Default::default()
            }),
        }
    }
}

impl UserAndGroupId {
    pub fn new(user_id: String, group_id: String) -> Self {
        Self { user_id, group_id }
    }
}

/*

impl SendGroupMsgRequest {
    pub fn new(msg: Msg, members_id: Vec<String>) -> Self {
        Self {
            message: Some(msg),
            members_id,
        }
    }

    pub fn new_with_group_invitation(
        send_id: String,
        msg: GroupInvitation,
        members_id: Vec<String>,
    ) -> Self {
        Self {
            message: Some(Msg {
                send_id,
                send_time: chrono::Local::now().timestamp_millis(),
                data: Some(Data::GroupInvitation(msg)),
                ..Default::default()
            }),
            members_id,
        }
    }

    pub fn new_with_group_msg(msg: Data, members_id: Vec<String>) -> Self {
        Self {
            message: Some(Msg {
                data: Some(msg),
                ..Default::default()
            }),
            members_id,
        }
    }


}

*/
