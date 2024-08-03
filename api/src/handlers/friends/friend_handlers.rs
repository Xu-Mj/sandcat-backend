use axum::extract::State;
use axum::Json;
use serde::Serialize;

use abi::errors::Error;
use abi::message::{
    AgreeReply, DeleteFriendRequest, Friend, FriendInfo, FriendshipWithUser, FsCreate,
    SendMsgRequest, UpdateRemarkRequest,
};

use crate::api_utils::custom_extract::{JsonWithAuthExtractor, PathWithAuthExtractor};
use crate::AppState;

#[derive(Serialize)]
pub struct FriendShipList {
    pub friends: Vec<Friend>,
    pub fs: Vec<FriendshipWithUser>,
}

pub async fn get_friends_list_by_user_id(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, offline_time)): PathWithAuthExtractor<(String, i64)>,
) -> Result<Json<FriendShipList>, Error> {
    let friends = app_state
        .db
        .friend
        .get_friend_list(&user_id, offline_time)
        .await?;
    let fs = app_state
        .db
        .friend
        .get_fs_list(&user_id, offline_time)
        .await?;
    Ok(Json(FriendShipList { friends, fs }))
}

// 获取好友申请列表
pub async fn get_apply_list_by_user_id(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, offline_time)): PathWithAuthExtractor<(String, i64)>,
) -> Result<Json<Vec<FriendshipWithUser>>, Error> {
    let list = app_state
        .db
        .friend
        .get_fs_list(&user_id, offline_time)
        .await?;
    Ok(Json(list))
}

// 创建好友请求
pub async fn create_friendship(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(new_friend): JsonWithAuthExtractor<FsCreate>,
) -> Result<Json<FriendshipWithUser>, Error> {
    tracing::debug!("{:?}", &new_friend);
    let receiver_id = new_friend.friend_id.clone();
    let (fs_req, fs_send) = app_state.db.friend.create_fs(new_friend).await?;

    // decode fs
    let fs = bincode::serialize(&fs_send)?;

    // increase send sequence
    let (cur_seq, _, _) = app_state.cache.incr_send_seq(&fs_send.user_id).await?;

    // send create fs message for online user
    let msg = SendMsgRequest::new_with_friend_ship_req(fs_send.user_id, receiver_id, fs, cur_seq);
    let mut chat_rpc = app_state.chat_rpc.clone();
    // need to send message to mq, because need to store
    chat_rpc.send_msg(msg).await?;
    Ok(Json(fs_req))
}

// 同意好友请求， 需要friend db参数
pub async fn agree(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(agree): JsonWithAuthExtractor<AgreeReply>,
) -> Result<Json<Friend>, Error> {
    // 同意好友添加请求，需要向同意方返回请求方的个人信息，向请求方发送同意方的个人信息
    let (req, send) = app_state
        .db
        .friend
        .agree_friend_apply_request(agree)
        .await?;

    let send_id = send.friend_id.clone();
    // decode friend
    let friend = bincode::serialize(&send)?;

    // increase send sequence
    let (cur_seq, _, _) = app_state.cache.incr_send_seq(&send_id).await?;

    // send message
    let mut chat_rpc = app_state.chat_rpc.clone();
    chat_rpc
        .send_msg(SendMsgRequest::new_with_friend_ship_resp(
            req.friend_id.clone(),
            friend,
            cur_seq,
        ))
        .await?;
    Ok(Json(req))
}

// 拉黑/取消拉黑
// pub async fn black_list(
//     State(app_state): State<AppState>,
//     JsonWithAuthExtractor(relation): JsonWithAuthExtractor<NewFriend>,
// ) -> Result<(), FriendError> {
//     update_friend_status(
//         &app_state.pool,
//         relation.user_id,
//         relation.friend_id,
//         relation.status,
//     )
//     .await
//     .map_err(|err| FriendError::InternalServerError(err.to_string()))?;
//     Ok(())
// }

pub async fn delete_friend(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<DeleteFriendRequest>,
) -> Result<(), Error> {
    req.validate()?;

    app_state
        .db
        .friend
        .delete_friend(&req.fs_id, &req.user_id)
        .await?;

    // send message to friend
    let msg = SendMsgRequest::new_with_friend_del(req.user_id, req.friend_id);
    let mut chat_rpc = app_state.chat_rpc.clone();
    chat_rpc.send_msg(msg).await?;
    Ok(())
}

pub async fn update_friend_remark(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(relation): JsonWithAuthExtractor<UpdateRemarkRequest>,
) -> Result<(), Error> {
    if relation.remark.is_empty() {
        return Err(Error::bad_request("remark is none"));
    }
    app_state
        .db
        .friend
        .update_friend_remark(&relation.user_id, &relation.friend_id, &relation.remark)
        .await?;
    Ok(())
}

pub async fn query_friend_info(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(user_id): PathWithAuthExtractor<String>,
) -> Result<Json<FriendInfo>, Error> {
    let user = app_state
        .db
        .user
        .get_user_by_id(&user_id)
        .await?
        .ok_or(Error::not_found())?;
    let friend = FriendInfo {
        id: user.id,
        name: user.name,
        region: user.region,
        gender: user.gender,
        avatar: user.avatar,
        account: user.account,
        signature: user.signature,
        email: user.email,
        age: user.age,
    };
    Ok(Json(friend))
}
