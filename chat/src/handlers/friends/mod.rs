pub mod friend_handlers;

use serde::Deserialize;

use crate::domain::model::friend_request_status::FriendStatus;

#[derive(Debug, Deserialize)]
pub struct FriendShipRequest {
    user_id: String,
    friend_id: String,
    status: FriendStatus,
    apply_msg: Option<String>,
    source: Option<String>,
    remark: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FriendShipAgree {
    friendship_id: String,
    response_msg: Option<String>,
    remark: Option<String>,
}
