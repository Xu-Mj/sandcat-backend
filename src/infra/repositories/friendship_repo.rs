use crate::domain::model::user::User;
use crate::infra::db::schema::{friendships, users};
use crate::infra::errors::{adapt_infra_error, InfraError};
use crate::infra::repositories::friends::{create_friend, FriendDb, FriendWithUser};
use crate::infra::repositories::user_repo::get;
use deadpool_diesel::postgres::Pool;
use diesel::pg::Pg;
use diesel::{
    AsChangeset, Associations, BoolExpressionMethods, ExpressionMethods, Insertable, JoinOnDsl,
    QueryDsl, Queryable, RunQueryDsl, Selectable, SelectableHelper,
};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

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
#[diesel(table_name=friendships)]
#[diesel(belongs_to(User))]
// 开启编译期字段检查，主要检查字段类型、数量是否匹配，可选
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FriendShipDb {
    pub id: String,
    pub user_id: String,
    pub friend_id: String,
    pub status: String,
    pub apply_msg: Option<String>,
    pub source: Option<String>,
    #[serde(default)]
    pub is_delivered: bool,
    #[serde(default)]
    pub create_time: chrono::NaiveDateTime,
    #[serde(default)]
    pub update_time: chrono::NaiveDateTime,
}

// 获取好友列表
pub async fn get_list_by_user_id(pool: &Pool, user_id: String) -> Result<Vec<User>, InfraError> {
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
#[derive(Serialize, Debug, Clone, Deserialize, Queryable)]
pub struct FriendShipWithUser {
    pub friendship_id: String,
    pub user_id: String,
    pub name: String,
    pub avatar: String,
    pub gender: String,
    pub age: i32,
    pub status: String,
    pub apply_msg: Option<String>,
    pub source: Option<String>,
    #[serde(default)]
    pub update_time: chrono::NaiveDateTime,
}

// 创建好友申请
pub async fn create_friend_ship(
    pool: &Pool,
    new_friend: FriendShipDb,
) -> Result<FriendShipWithUser, InfraError> {
    // 我想你发出申请，需要获取我的用户信息发送给你。
    let user_id = new_friend.user_id.clone();
    let friendship_id = new_friend.id.clone();
    let update_time = new_friend.update_time.clone();
    let apply_msg = new_friend.apply_msg.clone();
    let source = new_friend.source.clone();
    let status = new_friend.status.clone();
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    conn.interact(move |conn| {
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
            .execute(conn)
    })
    .await
    .map_err(adapt_infra_error)?
    .map_err(adapt_infra_error)?;
    let cloned_user_id = user_id.clone();
    let user = conn
        .interact(move |conn| {
            users::table
                .filter(users::id.eq(cloned_user_id))
                .select(User::as_select())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    let friend_ship = FriendShipWithUser {
        friendship_id,
        user_id,
        name: user.name,
        avatar: user.avatar,
        gender: user.gender,
        age: user.age,
        status,
        apply_msg,
        source,
        update_time,
    };
    Ok(friend_ship)
}

// 处理好友请求
pub async fn update_friend_ship(
    pool: &Pool,
    user_id: String,
    friend_id: String,
    status: String,
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
                    friendships::status.eq("1"),
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
pub async fn agree_apply(pool: &Pool, friendship_id: String) -> Result<FriendWithUser, InfraError> {
    let friendship = agree_friend_ship(pool, friendship_id).await?;
    let friend_id = friendship.user_id.clone();
    let now = chrono::Local::now().naive_local();
    let friends = vec![
        FriendDb {
            id: nanoid!(),
            user_id: friendship.user_id.clone(),
            friend_id: friendship.friend_id.clone(),
            status: "1".to_string(),
            remark: None,
            source: friendship.source.clone(),
            create_time: now,
            update_time: now,
        },
        FriendDb {
            id: nanoid!(),
            user_id: friendship.friend_id,
            friend_id: friendship.user_id,
            status: "1".to_string(),
            remark: None,
            source: friendship.source,
            create_time: now,
            update_time: now,
        },
    ];

    let _friend_db = create_friend(pool, friends).await?;
    let user = get(pool, friend_id).await?;
    tracing::debug!("friendship user: {:?}", &user);
    let friend = FriendWithUser {
        id: _friend_db.id,
        friend_id: user.id,
        remark: _friend_db.remark,
        status: _friend_db.status,
        create_time: _friend_db.create_time,
        update_time: _friend_db.update_time,
        from: _friend_db.source,
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
    Ok(friend)
}

// 根据用户id以及申请状态查询对应的记录
pub async fn get_by_user_id_and_status(
    pool: &Pool,
    user_id: String,
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
                .filter(friendships::status.ne("1"))
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
pub async fn msg_delivered(pool: &Pool, ids: Vec<String>) -> Result<usize, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    let count = conn
        .interact(|conn| {
            diesel::update(friendships::table)
                .filter(friendships::id.eq_any(ids))
                .set(friendships::is_delivered.eq(true))
                .execute(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(count)
}
