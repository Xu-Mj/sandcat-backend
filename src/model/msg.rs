use std::fmt::Formatter;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error, Visitor};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum ContentType {
    #[default]
    Default,
    Text,
    Image,
    Video,
    File,
    Emoji,
}

// 手动实现序列化接口，
// impl Serialize for ContentType {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
//         match self {
//             ContentType::Default => serializer.serialize_u8(0),
//             ContentType::Text => serializer.serialize_u8(1),
//             ContentType::Image => serializer.serialize_u8(2),
//             ContentType::Video => serializer.serialize_u8(3),
//             ContentType::File => serializer.serialize_u8(4),
//             ContentType::Emoji => serializer.serialize_u8(5),
//         }
//     }
// }


#[derive(Clone, Default, Copy, Debug, Serialize, Deserialize)]
pub enum MessageType {
    #[default]
    Default,
    Single,
    Group,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Msg {
    msg_type: MessageType,
    uuid: String,
    content: String,
    send_id: i32,
    friend_id: i32,
    content_type: ContentType,
    create_time: i64,
}

impl Msg {
    pub fn new(msg_type: MessageType,
               uuid: String,
               content: String,
               send_id: i32,
               friend_id: i32,
               content_type: ContentType,
               create_time: i64) -> Self {
        Self {
            msg_type,
            uuid,
            content,
            send_id,
            friend_id,
            content_type,
            create_time,
        }
    }
    pub fn send_id(&self) -> i32 {
        self.send_id
    }

    pub fn friend_id(&self) -> i32 {
        self.friend_id
    }

    pub fn msg_type(&self) -> MessageType {
        self.msg_type
    }
}
