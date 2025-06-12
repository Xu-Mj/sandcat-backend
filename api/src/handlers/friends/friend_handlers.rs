use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use abi::errors::Error;
use abi::message::{
    AgreeReply, DeleteFriendRequest, Friend, FriendGroup, FriendInfo, FriendPrivacySettings,
    FriendTag, FriendshipWithUser, FsCreate, ManageFriendTagRequest, SendMsgRequest,
    UpdateFriendGroupRequest, UpdateRemarkRequest,
};

use crate::AppState;
use crate::api_utils::custom_extract::{JsonWithAuthExtractor, PathWithAuthExtractor};

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

    let sender_id = send.friend_id.clone();
    // decode friend
    let friend = bincode::serialize(&send)?;

    // increase send sequence
    let (cur_seq, _, _) = app_state.cache.incr_send_seq(&sender_id).await?;

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

// 好友分组相关接口

// 好友分组管理
pub async fn create_friend_group(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<FriendGroup>,
) -> Result<Json<FriendGroup>, Error> {
    if req.name.is_empty() {
        return Err(Error::bad_request("group name is empty"));
    }

    let group = app_state
        .db
        .friend
        .create_friend_group(&req.user_id, &req.name, req.display_order)
        .await?;

    Ok(Json(group))
}

pub async fn update_friend_group(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<FriendGroup>,
) -> Result<Json<FriendGroup>, Error> {
    if req.name.is_empty() || req.id.is_empty() {
        return Err(Error::bad_request("group name or id is empty"));
    }

    let group = app_state
        .db
        .friend
        .update_friend_group(&req.id, &req.name, req.display_order)
        .await?;

    Ok(Json(group))
}

pub async fn delete_friend_group(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(id): PathWithAuthExtractor<String>,
) -> Result<(), Error> {
    app_state.db.friend.delete_friend_group(&id).await?;

    Ok(())
}

pub async fn get_friend_groups(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(id): PathWithAuthExtractor<String>,
) -> Result<Json<Vec<FriendGroup>>, Error> {
    let groups = app_state.db.friend.get_friend_groups(&id).await?;

    Ok(Json(groups))
}

pub async fn assign_friend_to_group(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<UpdateFriendGroupRequest>,
) -> Result<(), Error> {
    if req.friend_id.is_empty() || req.group_id.is_empty() {
        return Err(Error::bad_request("friend_id or group_id is empty"));
    }

    app_state
        .db
        .friend
        .assign_friend_to_group(&req.user_id, &req.friend_id, &req.group_id)
        .await?;

    Ok(())
}

// 好友标签管理
pub async fn create_friend_tag(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<FriendTag>,
) -> Result<Json<FriendTag>, Error> {
    if req.tag_name.is_empty() {
        return Err(Error::bad_request("tag name is empty"));
    }

    let tag = app_state
        .db
        .friend
        .create_friend_tag(&req.user_id, &req.tag_name, &req.tag_color)
        .await?;

    Ok(Json(tag))
}

pub async fn delete_friend_tag(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(id): PathWithAuthExtractor<String>,
) -> Result<(), Error> {
    app_state.db.friend.delete_friend_tag(&id).await?;

    Ok(())
}

pub async fn get_friend_tags(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(id): PathWithAuthExtractor<String>,
) -> Result<Json<Vec<FriendTag>>, Error> {
    let tags = app_state.db.friend.get_friend_tags(&id).await?;

    Ok(Json(tags))
}

pub async fn manage_friend_tags(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<ManageFriendTagRequest>,
) -> Result<(), Error> {
    if req.friend_id.is_empty() || req.tag_ids.is_empty() {
        return Err(Error::bad_request("friend_id or tag_ids is empty"));
    }

    if req.is_add {
        app_state
            .db
            .friend
            .add_tags_to_friend(&req.user_id, &req.friend_id, &req.tag_ids)
            .await?;
    } else {
        app_state
            .db
            .friend
            .remove_tags_from_friend(&req.user_id, &req.friend_id, &req.tag_ids)
            .await?;
    }

    Ok(())
}

// 好友隐私设置
pub async fn update_friend_privacy(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<FriendPrivacySettings>,
) -> Result<(), Error> {
    if req.friend_id.is_empty() {
        return Err(Error::bad_request("friend_id is empty"));
    }

    app_state
        .db
        .friend
        .update_friend_privacy(&req.user_id, &req.friend_id, &req)
        .await?;

    Ok(())
}

pub async fn get_friend_privacy(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, friend_id)): PathWithAuthExtractor<(String, String)>,
) -> Result<Json<FriendPrivacySettings>, Error> {
    let settings = app_state
        .db
        .friend
        .get_friend_privacy(&user_id, &friend_id)
        .await?;

    Ok(Json(settings))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionStats {
    pub score: f32,
    pub count: i32,
    pub last_interaction: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionScoreRequest {
    pub user_id: String,
    pub friend_id: String,
    pub score: f32,
}
// 好友互动分数
pub async fn update_interaction_score(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<InteractionScoreRequest>,
) -> Result<Json<f32>, Error> {
    if req.user_id.is_empty() || req.friend_id.is_empty() {
        return Err(Error::bad_request("user_id or friend_id is empty"));
    }

    let new_score = app_state
        .db
        .friend
        .update_interaction_score(&req.user_id, &req.friend_id, req.score)
        .await?;

    Ok(Json(new_score))
}

pub async fn get_interaction_stats(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, friend_id)): PathWithAuthExtractor<(String, String)>,
) -> Result<Json<InteractionStats>, Error> {
    let (score, count, last_interaction) = app_state
        .db
        .friend
        .get_interaction_stats(&user_id, &friend_id)
        .await?;

    let stats = InteractionStats {
        score,
        count,
        last_interaction,
    };

    Ok(Json(stats))
}
