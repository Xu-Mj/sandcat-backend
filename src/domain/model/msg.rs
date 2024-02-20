use crate::infra::repositories::friends::FriendWithUser;
use crate::infra::repositories::friendship_repo::{FriendShipDb, FriendShipWithUser, NewFriend};
use crate::infra::repositories::messages::MsgDb;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum ContentType {
    #[default]
    Default,
    Text,
    Image,
    Video,
    File,
    Emoji,
    Audio,
}

impl ContentType {
    pub fn from_str(value: &str) -> Self {
        match value {
            "Text" => Self::Text,
            "Image" => Self::Image,
            "Video" => Self::Video,
            "Audio" => Self::Audio,
            "File" => Self::File,
            "Emoji" => Self::Emoji,
            _ => Self::Default,
        }
    }
}

impl Display for ContentType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentType::Default => write!(f, "Default"),
            ContentType::Text => write!(f, "Text"),
            ContentType::Image => write!(f, "Image"),
            ContentType::Video => write!(f, "Video"),
            ContentType::File => write!(f, "File"),
            ContentType::Emoji => write!(f, "Emoji"),
            ContentType::Audio => write!(f, "Audio"),
        }
    }
}

#[derive(Clone, Default, Copy, Debug, Serialize, Deserialize)]
pub enum MessageType {
    #[default]
    Default,
    Single,
    Group,
    ReadNotice,
    DeliveredNotice,
}

impl MessageType {
    pub fn from_str(value: &str) -> Self {
        match value {
            "Single" => Self::Single,
            "Group" => Self::Group,
            "ReadNotice" => Self::ReadNotice,
            "DeliveredNotice" => Self::DeliveredNotice,
            _ => Self::Default,
        }
    }
}

impl Display for MessageType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Default => write!(f, "Default"),
            MessageType::Single => write!(f, "Single"),
            MessageType::Group => write!(f, "Group"),
            MessageType::ReadNotice => write!(f, "ReadNotice"),
            MessageType::DeliveredNotice => write!(f, "DeliveredNotice"),
        }
    }
}

impl Msg {
    pub fn single_from_db(msg: MsgDb) -> Self {
        let single = Single {
            msg_id: msg.msg_id,
            content: msg.content,
            send_id: msg.send_id,
            friend_id: msg.friend_id,
            content_type: ContentType::from_str(msg.content_type.as_str()),
            create_time: msg.create_time.timestamp_millis(),
        };
        Self::Single(single)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Msg {
    /// 一对一聊天
    Single(Single),
    /// 群聊
    Group(Single),
    /// 一对一通话offer
    SingleCallOffer(Offer),
    /// 一对一通话邀请
    SingleCallInvite(InviteMsg),
    /// 一对一通话邀请回复
    SingleCallInviteAnswer(InviteAnswerMsg),
    /// 一对一通话取消
    SingleCallInviteCancel(InviteCancelMsg),
    /// 一对一通话建立，被邀请方同意通话后，建立连接最后一步
    SingleCallAgree(Agree),
    /// 通话未接听
    SingleCallNotAnswer(InviteNotAnswerMsg),
    /// 挂断
    SingleCallHangUp(Hangup),
    /// 发送好友请求
    SendRelationshipReq(FriendShipDb),
    /// 收到好友请求，请求方发送SendRelationshipReq消息，转为RecRelationship后发给被请求方
    RecRelationship(FriendShipWithUser),
    /// 回复好友请求（同意）
    RelationshipRes(FriendWithUser),
    /// 消息已读
    ReadNotice(ReadNotice),
    /// 一对一消息送达
    SingleDeliveredNotice(DeliveredNotice),
    /// 好友请求送达
    FriendshipDeliveredNotice(DeliveredNotice),
    OfflineSync(Single),
    /// 通话协商消息
    NewIceCandidate(Candidate),
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Single {
    pub msg_id: String,
    pub content: String,
    pub send_id: String,
    pub friend_id: String,
    pub content_type: ContentType,
    pub create_time: i64,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_m_index: Option<u16>,

    pub send_id: String,
    pub friend_id: String,
    pub create_time: i64,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct InviteMsg {
    pub msg_id: String,
    pub send_id: String,
    pub friend_id: String,
    pub create_time: i64,
    pub invite_type: InviteType,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct InviteCancelMsg {
    pub msg_id: String,

    pub send_id: String,
    pub friend_id: String,
    pub create_time: i64,
    pub invite_type: InviteType,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct InviteNotAnswerMsg {
    pub msg_id: String,
    pub send_id: String,
    pub friend_id: String,
    pub create_time: i64,
    pub invite_type: InviteType,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub enum InviteType {
    Video,
    #[default]
    Audio,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct InviteAnswerMsg {
    pub msg_id: String,
    pub send_id: String,
    pub friend_id: String,
    pub create_time: i64,
    pub agree: bool,
    pub invite_type: InviteType,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Offer {
    pub sdp: String,
    pub send_id: String,
    pub friend_id: String,
    pub create_time: i64,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Agree {
    pub sdp: Option<String>,
    pub send_id: String,
    pub friend_id: String,
    pub create_time: i64,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Hangup {
    pub msg_id: String,
    pub send_id: String,
    pub friend_id: String,
    pub create_time: i64,
    pub invite_type: InviteType,
    pub sustain: i64,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Relation {
    pub send_id: String,
    pub friend_id: String,
    pub status: RelationStatus,
    pub create_time: i64,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ReadNotice {
    pub msg_ids: Vec<String>,
    pub send_id: String,
    pub friend_id: String,
    pub create_time: i64,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct DeliveredNotice {
    pub msg_id: String,
    pub create_time: i64,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum RelationStatus {
    #[default]
    Apply,
    Agree,
    Deny,
}
