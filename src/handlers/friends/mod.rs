use serde::Deserialize;

pub mod friend_handlers;

#[derive(Debug, Deserialize)]
pub struct FriendRequest {
    user_id: String,
    friend_id: String,
    status: String,
    apply_msg: Option<String>,
    source: Option<String>,
}
