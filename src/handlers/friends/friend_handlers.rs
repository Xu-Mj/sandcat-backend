// 根据用户id查询好友列表

use axum::extract::State;
use axum::Json;
use nanoid::nanoid;

use crate::domain::model::friend_request_status::FriendStatus;
use crate::domain::model::friends::FriendError;
use crate::domain::model::msg::Msg;
use crate::handlers::friends::{FriendShipAgree, FriendShipRequest};
use crate::infra::errors::InfraError;
use crate::infra::repositories::friends::{
    get_friend_list, update_friend_status, update_remark, FriendWithUser,
};
use crate::infra::repositories::friendship_repo::{
    agree_apply, create_friend_ship, get_by_user_id_and_status, update_friend_ship, FriendShipDb,
    FriendShipWithUser, NewFriend,
};
use crate::utils::{JsonWithAuthExtractor, PathWithAuthExtractor};
use crate::AppState;

// 获取好友列表
/*pub async fn get_friends_list_by_user_id(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(id): PathWithAuthExtractor<String>,
) -> Result<Json<Vec<User>>, FriendError> {
    let list = get_list_by_user_id(&app_state.pool, id.clone())
        .await
        .map_err(|err| match err {
            InfraError::InternalServerError(msg) => FriendError::InternalServerError(msg),
            InfraError::NotFound => FriendError::NotFound(id),
        })?;
    Ok(Json(list))
}
*/
pub async fn get_friends_list_by_user_id2(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(id): PathWithAuthExtractor<String>,
) -> Result<Json<Vec<FriendWithUser>>, FriendError> {
    let list = get_friend_list(&app_state.pool, id.clone())
        .await
        .map_err(|err| match err {
            InfraError::InternalServerError(msg) => FriendError::InternalServerError(msg),
            InfraError::NotFound => FriendError::NotFound(id),
        })?;
    Ok(Json(list))
}

// 获取好友申请列表
pub async fn get_apply_list_by_user_id(
    State(app_state): State<AppState>,
    PathWithAuthExtractor(id): PathWithAuthExtractor<String>,
) -> Result<Json<Vec<FriendShipWithUser>>, FriendError> {
    let list = get_by_user_id_and_status(&app_state.pool, id.clone(), FriendStatus::Pending)
        .await
        .map_err(|err| match err {
            InfraError::InternalServerError(msg) => FriendError::InternalServerError(msg),
            InfraError::NotFound => FriendError::NotFound(id),
        })?;
    Ok(Json(list))
}

// 创建好友请求
pub async fn create(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(new_friend): JsonWithAuthExtractor<FriendShipRequest>,
) -> Result<Json<FriendShipWithUser>, FriendError> {
    // 需要根据目标用户id查找是否在线，如果在线直接发送消息过去
    tracing::debug!("{:?}", &new_friend);
    let friend_id = new_friend.friend_id.clone();
    let (fs_req, fs_send) =
        create_friend_ship(&app_state.pool, get_friend_from_friend_req(new_friend))
            .await
            .map_err(|err| FriendError::InternalServerError(err.to_string()))?;
    // 查找在线用户
    app_state
        .hub
        .send_single_msg(&friend_id, &Msg::RecRelationship(fs_send))
        .await;
    // 用户id为好友id
    // friendship.user_id = friend_id;
    Ok(Json(fs_req))
}

pub fn get_friend_from_friend_req(friend: FriendShipRequest) -> FriendShipDb {
    FriendShipDb {
        id: nanoid!(),
        user_id: friend.user_id,
        friend_id: friend.friend_id,
        status: friend.status,
        apply_msg: friend.apply_msg,
        req_remark: friend.remark,
        response_msg: None,
        res_remark: None,
        source: friend.source,
        is_delivered: false,
        create_time: chrono::Local::now().naive_local(),
        update_time: chrono::Local::now().naive_local(),
    }
}

// 同意好友请求， 需要friend db参数
pub async fn agree(
    State(app_state): State<AppState>,
    // PathWithAuthExtractor(friendship_id): PathWithAuthExtractor<String>,
    JsonWithAuthExtractor(agree): JsonWithAuthExtractor<FriendShipAgree>,
) -> Result<Json<FriendWithUser>, FriendError> {
    // 同意好友添加请求，需要向同意方返回请求方的个人信息，向请求方发送同意方的个人信息

    let (user, friend) = agree_apply(
        &app_state.pool,
        agree.friendship_id,
        agree.response_msg,
        agree.remark,
    )
    .await
    .map_err(|err| FriendError::InternalServerError(err.to_string()))?;
    // 发送消息
    // 查找在线用户
    app_state
        .hub
        .send_single_msg(&user.friend_id, &Msg::RelationshipRes(friend))
        .await;
    Ok(Json(user))
}

// 拒绝好友请求
pub async fn deny(
    State(app_state): State<AppState>,
    PathWithAuthExtractor((user_id, friend_id)): PathWithAuthExtractor<(String, String)>,
) -> Result<(), FriendError> {
    update_friend_ship(&app_state.pool, user_id, friend_id, FriendStatus::Rejected)
        .await
        .map_err(|err| FriendError::InternalServerError(err.to_string()))?;
    Ok(())
}

// 拉黑/取消拉黑
pub async fn black_list(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(relation): JsonWithAuthExtractor<NewFriend>,
) -> Result<(), FriendError> {
    update_friend_status(
        &app_state.pool,
        relation.user_id,
        relation.friend_id,
        relation.status,
    )
    .await
    .map_err(|err| FriendError::InternalServerError(err.to_string()))?;
    Ok(())
}

pub async fn update_friend_remark(
    State(app_state): State<AppState>,
    JsonWithAuthExtractor(relation): JsonWithAuthExtractor<NewFriend>,
) -> Result<(), FriendError> {
    if relation.remark.is_none() {
        return Err(FriendError::Parameter(String::from("remark is none")));
    }
    update_remark(
        &app_state.pool,
        relation.user_id,
        relation.friend_id,
        relation.remark.unwrap(),
    )
    .await
    .map_err(|err| FriendError::InternalServerError(err.to_string()))?;
    Ok(())
}
