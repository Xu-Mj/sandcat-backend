use mongodb::bson::Document;
use tonic::Status;

use crate::errors::Error;
use crate::message::{GetDbMsgRequest, Msg, MsgResponse, MsgType, SendMsgRequest, UserAndGroupId};

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
            local_id: value.get_str("local_id").unwrap_or_default().to_string(),
            server_id: value.get_str("server_id").unwrap_or_default().to_string(),
            create_time: value.get_i64("create_time").unwrap_or_default(),
            send_time: value.get_i64("send_time").unwrap_or_default(),
            content_type: value.get_i32("content_type").unwrap_or_default(),
            content: value
                .get_binary_generic("content")
                .map_or(vec![], |v| v.to_vec()),
            send_id: value.get_str("send_id").unwrap_or_default().to_string(),
            receiver_id: value.get_str("receiver_id").unwrap_or_default().to_string(),
            seq: value.get_i64("seq").unwrap_or_default(),
            msg_type: value.get_i32("msg_type").unwrap_or_default(),
            is_read: value.get_bool("is_read").unwrap_or_default(),
            group_id: value.get_str("group_id").unwrap_or_default().to_string(),
            // those do not save to mongodb
            sdp: None,
            sdp_mid: None,
            sdp_m_index: None,
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
                group_id: receiver_id.clone(),
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
                group_id: receiver_id.clone(),
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
                group_id: receiver_id.clone(),
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
                group_id: receiver_id.clone(),
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

impl GetDbMsgRequest {
    pub fn validate(&self) -> Result<(), Error> {
        if self.user_id.is_empty() {
            return Err(Error::BadRequest("user_id is empty".to_string()));
        }
        if self.start < 0 {
            return Err(Error::BadRequest("start is invalid".to_string()));
        }
        if self.end < 0 {
            return Err(Error::BadRequest("end is invalid".to_string()));
        }
        if self.end < self.start {
            return Err(Error::BadRequest("start is greater than end".to_string()));
        }
        Ok(())
    }
}
