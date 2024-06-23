use axum::extract::State;
use axum::Json;

use abi::errors::Error;
use abi::message::{
    AgreeReply, DeleteFriendRequest, Friend, FriendInfo, FriendListRequest, FriendshipWithUser,
    FsAgreeRequest, FsCreate, FsCreateRequest, FsListRequest, QueryFriendInfoRequest,
    SendMsgRequest, UpdateRemarkRequest,
};

use crate::api_utils::custom_extract::{JsonWithAuthExtractor, PathWithAuthExtractor};
use crate::AppState;

pub async fn get_friends_list_by_user_id(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, offline_time)): PathWithAuthExtractor<(String, i64)>,
) -> Result<Json<Vec<Friend>>, Error> {
    let mut db_rpc = app_state.db_rpc.clone();
    let request = FriendListRequest {
        user_id,
        offline_time,
    };
    let response = db_rpc
        .get_friend_list(request)
        .await
        .map_err(|e| Error::InternalServer(e.to_string()))?;
    Ok(Json(response.into_inner().friends))
}

// 获取好友申请列表
pub async fn get_apply_list_by_user_id(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(user_id): PathWithAuthExtractor<String>,
) -> Result<Json<Vec<FriendshipWithUser>>, Error> {
    let mut db_rpc = app_state.db_rpc.clone();
    let response = db_rpc
        .get_friendship_list(FsListRequest { user_id })
        .await
        .map_err(|e| Error::InternalServer(e.to_string()))?;
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
        .await
        .map_err(|e| Error::InternalServer(e.to_string()))?;
    let inner = response.into_inner();
    let fs_req = inner
        .fs_req
        .ok_or_else(|| Error::InternalServer("create fs error".to_string()))?;

    let fs_send = inner
        .fs_send
        .ok_or_else(|| Error::InternalServer("send fs error".to_string()))?;

    // decode fs
    let fs = bincode::serialize(&fs_send).map_err(|e| Error::InternalServer(e.to_string()))?;
    // send create fs message for online user
    let msg = SendMsgRequest::new_with_friend_ship_req(fs_send.user_id, receiver_id, fs);
    let mut chat_rpc = app_state.chat_rpc.clone();
    // need to send message to mq, because need to store
    chat_rpc
        .send_msg(msg)
        .await
        .map_err(|e| Error::InternalServer(e.to_string()))?;
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
    let response = db_rpc
        .agree_friendship(request)
        .await
        .map_err(|e| Error::InternalServer(e.to_string()))?;
    let inner = response.into_inner();
    let req = inner
        .req
        .ok_or_else(|| Error::InternalServer("agree fs error".to_string()))?;
    let send = inner
        .send
        .ok_or_else(|| Error::InternalServer("send fs error".to_string()))?;
    // decode friend
    let friend = bincode::serialize(&send).map_err(|e| Error::InternalServer(e.to_string()))?;
    // send message
    let mut chat_rpc = app_state.chat_rpc.clone();
    chat_rpc
        .send_msg(SendMsgRequest::new_with_friend_ship_resp(
            req.friend_id.clone(),
            friend,
        ))
        .await
        .map_err(|e| Error::InternalServer(e.to_string()))?;
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
    if req.user_id.is_empty() {
        return Err(Error::BadRequest(String::from("user id is none")));
    }
    if req.friend_id.is_empty() {
        return Err(Error::BadRequest(String::from("friend id is none")));
    }

    let user_id = req.user_id.clone();
    let friend_id = req.friend_id.clone();

    let mut db_rpc = app_state.db_rpc.clone();
    db_rpc
        .delete_friend(req)
        .await
        .map_err(|e| Error::InternalServer(e.to_string()))?;

    // send message to friend
    let msg = SendMsgRequest::new_with_friend_del(user_id, friend_id);
    let mut chat_rpc = app_state.chat_rpc.clone();
    chat_rpc
        .send_msg(msg)
        .await
        .map_err(|e| Error::InternalServer(e.to_string()))?;
    Ok(())
}

pub async fn update_friend_remark(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(relation): JsonWithAuthExtractor<UpdateRemarkRequest>,
) -> Result<(), Error> {
    if relation.remark.is_empty() {
        return Err(Error::BadRequest(String::from("remark is none")));
    }
    let mut db_rpc = app_state.db_rpc.clone();
    db_rpc
        .update_friend_remark(relation)
        .await
        .map_err(|e| Error::InternalServer(e.to_string()))?;
    Ok(())
}

pub async fn query_friend_info(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(user_id): PathWithAuthExtractor<String>,
) -> Result<Json<FriendInfo>, Error> {
    let mut db_rpc = app_state.db_rpc.clone();
    let friend = db_rpc
        .query_friend_info(QueryFriendInfoRequest { user_id })
        .await
        .map_err(|e| Error::InternalServer(e.to_string()))?;
    let friend = friend.into_inner().friend.ok_or(Error::NotFound)?;
    Ok(Json(friend))
}
