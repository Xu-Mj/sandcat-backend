use axum::extract::State;
use axum::Json;
use serde::Serialize;

use abi::errors::Error;
use abi::message::{
    AgreeReply, DeleteFriendRequest, Friend, FriendInfo, FriendListRequest, FriendshipWithUser,
    FsAgreeRequest, FsCreate, FsCreateRequest, QueryFriendInfoRequest, SendMsgRequest,
    UpdateRemarkRequest,
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
    let mut db_rpc = app_state.db_rpc.clone();
    let request = FriendListRequest {
        user_id,
        offline_time,
    };
    let friends = db_rpc
        .get_friend_list(request.clone())
        .await?
        .into_inner()
        .friends;
    let fs = db_rpc
        .get_friendship_list(request)
        .await?
        .into_inner()
        .friendships;
    Ok(Json(FriendShipList { friends, fs }))
}

// 获取好友申请列表
pub async fn get_apply_list_by_user_id(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, offline_time)): PathWithAuthExtractor<(String, i64)>,
) -> Result<Json<Vec<FriendshipWithUser>>, Error> {
    let mut db_rpc = app_state.db_rpc.clone();
    let response = db_rpc
        .get_friendship_list(FriendListRequest {
            user_id,
            offline_time,
        })
        .await?;
    let list = response.into_inner().friendships;
    Ok(Json(list))
}

// 创建好友请求
pub async fn create_friendship(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(new_friend): JsonWithAuthExtractor<FsCreate>,
) -> Result<Json<FriendshipWithUser>, Error> {
    tracing::debug!("{:?}", &new_friend);
    let receiver_id = new_friend.friend_id.clone();
    let mut db_rpc = app_state.db_rpc.clone();
    let response = db_rpc
        .create_friendship(FsCreateRequest {
            fs_create: Some(new_friend),
        })
        .await?;
    let inner = response.into_inner();
    let fs_req = inner
        .fs_req
        .ok_or(Error::internal_with_details("create fs error"))?;

    let fs_send = inner
        .fs_send
        .ok_or(Error::internal_with_details("send fs error"))?;

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
    let mut db_rpc = app_state.db_rpc.clone();
    let request = FsAgreeRequest {
        fs_reply: Some(agree),
    };
    let response = db_rpc.agree_friendship(request).await?;
    let inner = response.into_inner();
    let req = inner
        .req
        .ok_or(Error::internal_with_details("agree fs error"))?;
    let send = inner
        .send
        .ok_or(Error::internal_with_details("send fs error"))?;

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

    let user_id = req.user_id.clone();
    let friend_id = req.friend_id.clone();

    let mut db_rpc = app_state.db_rpc.clone();
    db_rpc.delete_friend(req).await?;

    // send message to friend
    let msg = SendMsgRequest::new_with_friend_del(user_id, friend_id);
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
    let mut db_rpc = app_state.db_rpc.clone();
    db_rpc.update_friend_remark(relation).await?;
    Ok(())
}

pub async fn query_friend_info(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(user_id): PathWithAuthExtractor<String>,
) -> Result<Json<FriendInfo>, Error> {
    let mut db_rpc = app_state.db_rpc.clone();
    let friend = db_rpc
        .query_friend_info(QueryFriendInfoRequest { user_id })
        .await?;
    let friend = friend.into_inner().friend.ok_or(Error::not_found())?;
    Ok(Json(friend))
}
