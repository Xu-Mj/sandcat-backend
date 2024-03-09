use deadpool_diesel::postgres::Pool;
use diesel::pg::Pg;
use diesel::{
    AsChangeset, Associations, BoolExpressionMethods, ExpressionMethods, Insertable, JoinOnDsl,
    QueryDsl, Queryable, RunQueryDsl, Selectable, SelectableHelper,
};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::domain::model::friend_request_status::FriendStatus;
use crate::domain::model::friends::{FriendDb, FriendWithUser};
use crate::domain::model::user::User;
use crate::infra::db::schema::{friendships, users};
use crate::infra::errors::{adapt_infra_error, InfraError};
use crate::infra::repositories::friends::create_friend;
use crate::infra::repositories::user_repo::get_by_2id;

// status: 0-解除好友关系；1-同意请求；2-申请；3-拒绝

#[derive(
    Serialize,
    Queryable,
    Selectable,
    Associations,
    Debug,
    Insertable,
    Clone,
    Deserialize,
    AsChangeset,
)]
#[diesel(table_name = friendships)]
#[diesel(belongs_to(User))]
// 开启编译期字段检查，主要检查字段类型、数量是否匹配，可选
#[diesel(check_for_backend(diesel::pg::Pg))]
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

// 获取好友列表
/*pub async fn get_list_by_user_id(pool: &Pool, user_id: String) -> Result<Vec<User>, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    let users = conn
        .interact(|conn| {
            let friends_ids = friendships::table
                .filter(
                    friendships::user_id
                        .eq(user_id.clone())
                        .and(friendships::status.eq("1")),
                )
                .select(friendships::friend_id);
            let friendships_ids2 = friendships::table
                .filter(
                    friendships::friend_id
                        .eq(user_id)
                        .and(friendships::status.eq("1")),
                )
                .select(friendships::user_id);
            users::table
                .filter(
                    users::id
                        .eq_any(friends_ids)
                        .or(users::id.eq_any(friendships_ids2)),
                )
                .load::<User>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(users)
}
*/
/// for friendship
#[derive(Serialize, Debug, Clone, Deserialize, Queryable)]
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

// 创建好友申请
pub async fn create_friend_ship(
    pool: &Pool,
    new_friend: FriendShipDb,
) -> Result<(FriendShipWithUser, FriendShipWithUser), InfraError> {
    // 我想你发出申请，需要获取我的用户信息发送给你。
    // 需要查询两个人的个人信息
    // 需要给请求方返回对方的个人信息，给被请求方发送请求方的个人信息
    let user_id = new_friend.user_id.clone();
    let friend_id = new_friend.friend_id.clone();
    // let friendship_id = new_friend.id.clone();
    let update_time = new_friend.update_time;
    let apply_msg = new_friend.apply_msg.clone();
    let source = new_friend.source.clone();
    let status = new_friend.status.clone();
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    let friendship: FriendShipDb = conn
        .interact(move |conn| {
            // 如果已经存在一条请求信息，那么执行更新操作
            diesel::insert_into(friendships::table)
                .values(&new_friend)
                .on_conflict((friendships::user_id, friendships::friend_id))
                .do_update()
                .set((
                    friendships::status.eq(&new_friend.status),
                    friendships::apply_msg.eq(&new_friend.apply_msg),
                    friendships::source.eq(&new_friend.source),
                    friendships::update_time.eq(&new_friend.update_time),
                    friendships::is_delivered.eq(false),
                ))
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    let cloned_user_id = user_id.clone();
    let cloned_friend_id = friend_id.clone();
    let mut users = conn
        .interact(move |conn| {
            users::table
                .filter(users::id.eq_any(vec![cloned_user_id, cloned_friend_id]))
                .select(User::as_select())
                .get_results(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    tracing::debug!("users : {:?}", &users);
    let user1 = users.remove(0);
    let (user, friend) = if user1.id == user_id.clone() {
        (user1, users.remove(0))
    } else {
        (users.remove(0), user1)
    };
    // 发送给被请求方，如果不在线那么丢弃
    let fs_send = FriendShipWithUser {
        friendship_id: friendship.id.clone(),
        user_id,
        name: user.name,
        avatar: user.avatar,
        gender: user.gender,
        age: user.age,
        status: status.clone(),
        apply_msg: apply_msg.clone(),
        source: source.clone(),
        update_time,
    };
    // 返回给请求方
    let fs_req = FriendShipWithUser {
        friendship_id: friendship.id,
        user_id: friend_id,
        name: friend.name,
        avatar: friend.avatar,
        gender: friend.gender,
        age: friend.age,
        status,
        apply_msg,
        source,
        update_time,
    };
    tracing::debug!(
        " create friend ship, req {:?}; send: {:?}",
        &fs_req,
        &fs_send
    );
    Ok((fs_req, fs_send))
}

// 处理好友请求
pub async fn update_friend_ship(
    pool: &Pool,
    user_id: String,
    friend_id: String,
    status: FriendStatus,
) -> Result<FriendShipDb, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;

    let friend_ship = conn
        .interact(|conn| {
            diesel::update(friendships::table)
                .filter(
                    friendships::user_id
                        .eq(user_id)
                        .and(friendships::friend_id.eq(friend_id)),
                )
                .set(friendships::status.eq(status))
                .returning(FriendShipDb::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(friend_ship)
}

// 同意好友请求
pub async fn agree_friend_ship(
    pool: &Pool,
    friendship_id: String,
    response_msg: Option<String>,
    res_remark: Option<String>,
) -> Result<FriendShipDb, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;

    let friend_ship = conn
        .interact(|conn| {
            diesel::update(friendships::table)
                .filter(friendships::id.eq(friendship_id))
                .set((
                    friendships::status.eq(FriendStatus::Accepted),
                    friendships::response_msg.eq(response_msg),
                    friendships::res_remark.eq(res_remark),
                    friendships::is_delivered.eq(false),
                ))
                .returning(FriendShipDb::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(friend_ship)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NewFriend {
    pub user_id: String,
    pub friend_id: String,
    pub status: String,
    pub remark: Option<String>,
    pub source: Option<String>,
}

/// Result<(FriendWithUser, FriendWithUser), InfraError>
/// friend1返回给调用方，friend2发送给friend1.friend_id
pub async fn agree_apply(
    pool: &Pool,
    friendship_id: String,
    response_msg: Option<String>,
    res_remark: Option<String>,
) -> Result<(FriendWithUser, FriendWithUser), InfraError> {
    // FIXME 需要使用事务将两次数据库修改操作绑定
    let friendship =
        agree_friend_ship(pool, friendship_id.clone(), response_msg, res_remark).await?;
    // 添加好友请求方
    let user_id = friendship.user_id.clone();
    // 被请求方，返回给本次http请求方
    let friend_id = friendship.friend_id.clone();
    let now = chrono::Local::now().naive_local();
    let friends = vec![
        // 添加好友请求方
        FriendDb {
            id: nanoid!(),
            friendship_id: friendship_id.clone(),
            user_id: friendship.user_id.clone(),
            friend_id: friendship.friend_id.clone(),
            status: FriendStatus::Accepted,
            remark: friendship.req_remark.clone(),
            hello: friendship.response_msg.clone(),
            source: friendship.source.clone(),
            create_time: now,
            update_time: now,
        },
        // 添加好友被请求方
        FriendDb {
            id: nanoid!(),
            friendship_id: friendship_id.clone(),
            user_id: friendship.friend_id.clone(),
            friend_id: friendship.user_id.clone(),
            status: FriendStatus::Accepted,
            remark: friendship.res_remark.clone(),
            hello: friendship.apply_msg.clone(),
            source: friendship.source.clone(),
            create_time: now,
            update_time: now,
        },
    ];

    // 创建好友记录，
    create_friend(pool, friends).await?;
    // user为好友添加请求方， friend为被请求方
    let (user, friend) = get_by_2id(pool, user_id, friend_id).await?;
    tracing::debug!("friendship user: {:?}", &user);
    // 返回给本次http请求方
    let friend1 = FriendWithUser {
        id: friendship_id.clone(),
        friend_id: user.id,
        remark: friendship.res_remark,
        // 这里的hello是我们发给对方的消息
        hello: friendship.response_msg,
        status: friendship.status.clone(),
        create_time: now,
        update_time: now,
        from: friendship.source.clone(),
        name: user.name,
        account: user.account,
        avatar: user.avatar,
        gender: user.gender,
        age: user.age,
        phone: user.phone,
        email: user.email,
        address: user.address,
        birthday: user.birthday,
    };
    // 发送给好友添加请求方
    let friend2 = FriendWithUser {
        id: friendship_id,
        friend_id: friend.id,
        remark: friendship.req_remark,
        // 这里的hello是我们发给对方的消息
        hello: friendship.apply_msg,
        status: friendship.status,
        create_time: now,
        update_time: now,
        from: friendship.source,
        name: friend.name,
        account: friend.account,
        avatar: friend.avatar,
        gender: friend.gender,
        age: friend.age,
        phone: friend.phone,
        email: friend.email,
        address: friend.address,
        birthday: friend.birthday,
    };
    Ok((friend1, friend2))
}

/// 根据用户id查询好友请求同意记录
pub async fn get_agree_by_user_id(
    pool: &PgPool,
    user_id: String,
) -> Result<Vec<FriendWithUser>, InfraError> {
    // query friendship table first
    // query friend table by friend id of friendship query result
    let friends: Vec<FriendWithUser> =
        sqlx::query_as("
        SELECT
        f.id, f.friend_id, f.req_remark as remark, f.apply_msg as hello, f.status, f.create_time, f.update_time, f.source as from,
        u.name, u.account, u.avatar, u.gender, u.age, u.phone, u.email, u.address, u.birthday
        FROM  friendships f
        INNER JOIN users u ON u.id = f.friend_id
        WHERE f.user_id = $1 AND f.status = 'Accepted' AND f.is_delivered = false")
        .bind(&user_id)
        .fetch_all(pool)
        .await?;
    Ok(friends)
}
// 根据用户id以及申请状态查询对应的记录
pub async fn get_by_user_id_and_status(
    pool: &Pool,
    user_id: String,
    status: FriendStatus,
) -> Result<Vec<FriendShipWithUser>, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    let users = conn
        .interact(move |conn| {
            let statement = friendships::table
                .inner_join(users::table.on(users::id.eq(friendships::user_id)))
                .filter(friendships::friend_id.eq(user_id.clone()))
                .filter(friendships::status.eq(status))
                .filter(friendships::is_delivered.eq(false))
                .select((
                    friendships::id,
                    friendships::user_id,
                    users::name,
                    users::avatar,
                    users::gender,
                    users::age,
                    friendships::status,
                    friendships::apply_msg,
                    friendships::source,
                    friendships::update_time,
                ));
            tracing::debug!(
                "get friendship list: {:?}",
                diesel::debug_query::<Pg, _>(&statement)
            );
            statement.load::<FriendShipWithUser>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(users)
}

// 消息送达
pub async fn msg_delivered(pool: &PgPool, msg_id: &str) -> Result<(), InfraError> {
    sqlx::query("UPDATE friendships SET is_delivered = true, update_time = now() WHERE id = $1")
        .bind(msg_id)
        .execute(pool)
        .await?;
    Ok(())
}
