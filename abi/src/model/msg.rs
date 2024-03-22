use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::model::friend_request_status::FriendStatus;
use crate::model::friends::FriendWithUser;
use crate::model::group_members::GroupMemberWithUser;

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
    VideoCall,
    AudioCall,
}

impl FromStr for ContentType {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "Text" => Self::Text,
            "Image" => Self::Image,
            "Video" => Self::Video,
            "Audio" => Self::Audio,
            "File" => Self::File,
            "Emoji" => Self::Emoji,
            "VideoCall" => Self::VideoCall,
            "AudioCall" => Self::AudioCall,
            _ => Self::Default,
        })
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
            ContentType::VideoCall => write!(f, "VideoCall"),
            ContentType::AudioCall => write!(f, "AudioCall"),
        }
    }
}

/* #[derive(Clone, Default, Copy, Debug, Serialize, Deserialize)]
pub enum MessageType {
    #[default]
    Default,
    Single,
    Group,
    ReadNotice,
    DeliveredNotice,
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
 */
/*impl Msg {
    pub fn single_from_db(msg: MsgDb) -> Self {
        let single = Single {
            msg_id: msg.msg_id,
            content: msg.content,
            send_id: msg.send_id,
            receiver_id: msg.friend_id,
            content_type: ContentType::from_str(msg.content_type.as_str()),
            create_time: msg.create_time.timestamp_millis(),
        };
        Self::Single(single)
    }
}*/
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupDb {
    pub id: String,
    pub owner: String,
    pub name: String,
    pub avatar: String,
    pub description: String,
    pub announcement: String,
    pub create_time: chrono::NaiveDateTime,
    pub update_time: chrono::NaiveDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupInvitation {
    pub info: GroupDb,
    pub members: Vec<GroupMemberWithUser>,
}
/*
impl From<GroupRequest> for GroupInvitation {
    fn from(value: GroupRequest) -> Self {
        Self {
            info: GroupDb::from(value),
            members: vec![],
        }
    }
}
*/
/* #[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct GroupInfo {
    pub id: String,
    pub owner: String,
    pub avatar: String,
    pub group_name: String,
    pub create_time: i64,
    pub announcement: String,
}
 */
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendShipDb {
    pub id: String,
    pub user_id: String,
    pub friend_id: String,
    pub status: FriendStatus,
    pub apply_msg: Option<String>,
    pub req_remark: Option<String>,
    pub response_msg: Option<String>,
    pub res_remark: Option<String>,
    pub source: Option<String>,
    #[serde(default)]
    pub is_delivered: bool,
    #[serde(default)]
    pub create_time: chrono::NaiveDateTime,
    #[serde(default)]
    pub update_time: chrono::NaiveDateTime,
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct FriendShipWithUser {
    pub friendship_id: String,
    pub user_id: String,
    pub name: String,
    pub avatar: String,
    pub gender: String,
    pub age: i32,
    pub status: FriendStatus,
    pub apply_msg: Option<String>,
    pub source: Option<String>,
    #[serde(default)]
    pub update_time: chrono::NaiveDateTime,
}

pub type MessageID = String;
pub type GroupID = String;
pub type UserID = String;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Msg {
    /// 一对一聊天
    Single(Single),
    /// 群聊
    Group(GroupMsg),
    // GroupInvitation(GroupInvitation),
    // GroupInvitationReceived((UserID, GroupID)),
    /// 发送好友请求
    SendRelationshipReq(FriendShipDb),
    /// 收到好友请求，请求方发送SendRelationshipReq消息，转为RecRelationship后发给被请求方
    RecRelationship(FriendShipWithUser),
    /// 回复好友请求（同意）
    RelationshipRes(FriendWithUser),
    /// 消息已读
    ReadNotice(ReadNotice),
    /// 一对一消息送达
    SingleDeliveredNotice(MessageID),
    /// 好友请求送达
    FriendshipDeliveredNotice(MessageID),
    OfflineSync(Single),
    SingleCall(SingleCall),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GroupMsg {
    Message(Single),
    Invitation(GroupInvitation),
    MemberExit((UserID, GroupID)),
    Dismiss(GroupID),
    DismissOrExitReceived((UserID, GroupID)),
    InvitationReceived((UserID, GroupID)),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SingleCall {
    /// 一对一通话offer
    Offer(Offer),
    /// 一对一通话邀请
    Invite(InviteMsg),
    /// 一对一通话邀请回复
    InviteAnswer(InviteAnswerMsg),
    /// 一对一通话取消
    InviteCancel(InviteCancelMsg),
    /// 一对一通话建立，被邀请方同意通话后，建立连接最后一步
    Agree(Agree),
    /// 通话未接听
    NotAnswer(InviteNotAnswerMsg),
    /// 挂断
    HangUp(Hangup),
    /// 通话协商消息
    NewIceCandidate(Candidate),
}

impl SingleCall {
    pub fn get_friend_id(&self) -> Option<&str> {
        match self {
            SingleCall::Offer(msg) => Some(&msg.friend_id),
            SingleCall::Invite(msg) => Some(&msg.friend_id),
            SingleCall::InviteAnswer(msg) => Some(&msg.friend_id),
            SingleCall::InviteCancel(msg) => Some(&msg.friend_id),
            SingleCall::Agree(msg) => Some(&msg.friend_id),
            SingleCall::NotAnswer(msg) => Some(&msg.friend_id),
            SingleCall::HangUp(msg) => Some(&msg.friend_id),
            SingleCall::NewIceCandidate(msg) => Some(&msg.friend_id),
        }
    }
}

impl Msg {
    pub fn get_friend_id(&self) -> Option<&str> {
        match self {
            Msg::Single(single) => Some(&single.receiver_id),
            Msg::Group(GroupMsg::Message(single)) => Some(&single.receiver_id),
            Msg::SingleCall(msg) => msg.get_friend_id(),

            _ => None,
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Single {
    pub msg_id: String,
    pub content: String,
    pub send_id: String,
    pub receiver_id: String,
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

// #[derive(Clone, Default, Debug, Serialize, Deserialize)]
// pub struct DeliveredNotice {
//     pub msg_id: String,
//     pub create_time: i64,
// }

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum RelationStatus {
    #[default]
    Apply,
    Agree,
    Deny,
}
