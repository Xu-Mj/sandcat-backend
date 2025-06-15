use mongodb::bson::Document;
use tonic::Status;

use crate::errors::Error;
use crate::message::{
    GetDbMessagesRequest, GetDbMsgRequest, GroupMemSeq, Msg, MsgResponse, MsgType,
    SaveGroupMsgRequest, SaveMessageRequest, SendMsgRequest, UserAndGroupId,
};

impl From<Status> for MsgResponse {
    fn from(status: Status) -> Self {
        MsgResponse {
            client_id: String::new(),
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
            client_id: value.get_str("client_id").unwrap_or_default().to_string(),
            server_id: value.get_str("server_id").unwrap_or_default().to_string(),
            create_time: value.get_i64("create_time").unwrap_or_default(),
            send_time: value.get_i64("send_time").unwrap_or_default(),
            content_type: value.get_i32("content_type").unwrap_or_default(),
            content: value
                .get_binary_generic("content")
                .map_or(vec![], |v| v.to_vec()),
            sender_id: value.get_str("sender_id").unwrap_or_default().to_string(),
            receiver_id: value.get_str("receiver_id").unwrap_or_default().to_string(),
            seq: value.get_i64("seq").unwrap_or_default(),
            send_seq: value.get_i64("send_seq").unwrap_or_default(),
            msg_type: value.get_i32("msg_type").unwrap_or_default(),
            is_read: value.get_bool("is_read").unwrap_or_default(),
            group_id: value.get_str("group_id").unwrap_or_default().to_string(),
            platform: value.get_i32("platform").unwrap_or_default(),
            avatar: value.get_str("avatar").unwrap_or_default().to_string(),
            nickname: value.get_str("nickname").unwrap_or_default().to_string(),
            related_msg_id: value
                .get_str("related_msg_id")
                .map_or(None, |v| Some(v.to_string())),
        })
    }
}

impl SendMsgRequest {
    pub fn new_with_friend_del(sender_id: String, receiver_id: String) -> Self {
        Self {
            message: Some(Msg {
                sender_id,
                receiver_id,
                send_time: chrono::Utc::now().timestamp_millis(),
                msg_type: MsgType::FriendDelete as i32,
                ..Default::default()
            }),
        }
    }

    pub fn new_with_friend_ship_req(
        sender_id: String,
        receiver_id: String,
        fs: Vec<u8>,
        send_seq: i64,
    ) -> Self {
        Self {
            message: Some(Msg {
                send_seq,
                sender_id,
                receiver_id,
                send_time: chrono::Utc::now().timestamp_millis(),
                content: fs,
                msg_type: MsgType::FriendApplyReq as i32,
                ..Default::default()
            }),
        }
    }

    pub fn new_with_friend_ship_resp(receiver_id: String, fs: Vec<u8>, send_seq: i64) -> Self {
        Self {
            message: Some(Msg {
                send_seq,
                receiver_id,
                content: fs,
                msg_type: MsgType::FriendApplyResp as i32,
                send_time: chrono::Utc::now().timestamp_millis(),
                ..Default::default()
            }),
        }
    }

    /// when dismiss group, send id is the owner id,
    /// when member exit group, send id is the member id
    pub fn new_with_group_operation(
        sender_id: String,
        receiver_id: String,
        msg_type: MsgType,
        send_seq: i64,
    ) -> Self {
        Self {
            message: Some(Msg {
                sender_id,
                group_id: receiver_id.clone(),
                receiver_id,
                send_time: chrono::Utc::now().timestamp_millis(),
                msg_type: msg_type as i32,
                send_seq,
                ..Default::default()
            }),
        }
    }

    pub fn new_with_group_invitation(
        sender_id: String,
        receiver_id: String,
        send_seq: i64,
        invitation: Vec<u8>,
    ) -> Self {
        Self {
            message: Some(Msg {
                sender_id,
                group_id: receiver_id.clone(),
                receiver_id,
                send_time: chrono::Utc::now().timestamp_millis(),
                msg_type: MsgType::GroupInvitation as i32,
                content: invitation,
                send_seq,
                ..Default::default()
            }),
        }
    }

    pub fn new_with_group_invite_new(
        sender_id: String,
        receiver_id: String,
        send_seq: i64,
        invitation: Vec<u8>,
    ) -> Self {
        Self {
            message: Some(Msg {
                sender_id,
                group_id: receiver_id.clone(),
                receiver_id,
                send_time: chrono::Utc::now().timestamp_millis(),
                msg_type: MsgType::GroupInviteNew as i32,
                content: invitation,
                send_seq,
                ..Default::default()
            }),
        }
    }

    pub fn new_with_group_remove_mem(
        sender_id: String,
        group_id: String,
        send_seq: i64,
        invitation: Vec<u8>,
    ) -> Self {
        Self {
            message: Some(Msg {
                sender_id,
                receiver_id: group_id.clone(),
                group_id,
                send_time: chrono::Utc::now().timestamp_millis(),
                msg_type: MsgType::GroupRemoveMember as i32,
                content: invitation,
                send_seq,
                ..Default::default()
            }),
        }
    }

    pub fn new_with_group_update(
        sender_id: String,
        receiver_id: String,
        send_seq: i64,
        msg: Vec<u8>,
    ) -> Self {
        Self {
            message: Some(Msg {
                sender_id,
                group_id: receiver_id.clone(),
                receiver_id,
                send_time: chrono::Utc::now().timestamp_millis(),
                msg_type: MsgType::GroupUpdate as i32,
                content: msg,
                send_seq,
                ..Default::default()
            }),
        }
    }

    // 群组文件消息
    pub fn new_with_group_file(
        sender: String,
        group_id: String,
        seq: i64,
        content: Vec<u8>,
    ) -> Self {
        Self {
            message: Some(Msg {
                sender_id: sender,
                group_id,
                seq,
                content,
                msg_type: MsgType::GroupFile as i32,
                ..Default::default()
            }),
        }
    }

    // 群组投票消息
    pub fn new_with_group_poll(
        sender: String,
        group_id: String,
        seq: i64,
        content: Vec<u8>,
    ) -> Self {
        Self {
            message: Some(Msg {
                sender_id: sender,
                group_id,
                seq,
                content,
                msg_type: MsgType::GroupPoll as i32,
                ..Default::default()
            }),
        }
    }

    // 群组禁言消息
    pub fn new_with_group_mute(
        sender: String,
        group_id: String,
        seq: i64,
        content: Vec<u8>,
    ) -> Self {
        Self {
            message: Some(Msg {
                sender_id: sender,
                group_id,
                seq,
                content,
                msg_type: MsgType::GroupMute as i32,
                ..Default::default()
            }),
        }
    }

    // 群组公告消息
    pub fn new_with_group_announcement(
        sender: String,
        group_id: String,
        seq: i64,
        content: Vec<u8>,
    ) -> Self {
        Self {
            message: Some(Msg {
                sender_id: sender,
                group_id,
                seq,
                content,
                msg_type: MsgType::GroupAnnouncement as i32,
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
            return Err(Error::bad_request("user_id is empty"));
        }
        if self.start < 0 {
            return Err(Error::bad_request("start is invalid"));
        }
        if self.end < 0 {
            return Err(Error::bad_request("end is invalid"));
        }
        if self.end < self.start {
            return Err(Error::bad_request("start is greater than end"));
        }
        Ok(())
    }
}

impl GetDbMessagesRequest {
    pub fn validate(&self) -> Result<(), Error> {
        if self.user_id.is_empty() {
            return Err(Error::bad_request("user_id is empty"));
        }
        if self.start < 0 {
            return Err(Error::bad_request("start is invalid"));
        }
        if self.end < 0 {
            return Err(Error::bad_request("end is invalid"));
        }
        if self.end < self.start {
            return Err(Error::bad_request("start is greater than end"));
        }
        Ok(())
    }
}

impl SaveMessageRequest {
    pub fn new(msg: Msg, need_to_history: bool) -> Self {
        Self {
            message: Some(msg),
            need_to_history,
        }
    }
}

impl SaveGroupMsgRequest {
    pub fn new(msg: Msg, need_to_history: bool, members: Vec<GroupMemSeq>) -> Self {
        Self {
            message: Some(msg),
            need_to_history,
            members,
        }
    }
}
