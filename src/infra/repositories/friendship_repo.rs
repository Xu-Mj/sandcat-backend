use crate::domain::model::user::User;
use crate::infra::db::schema::{friendships, users};
use crate::infra::errors::{adapt_infra_error, InfraError};
use crate::infra::repositories::friends::{create_friend, FriendDb};
use deadpool_diesel::postgres::Pool;
use diesel::{
    BoolExpressionMethods, ExpressionMethods, Insertable, QueryDsl, Queryable, RunQueryDsl,
    Selectable, SelectableHelper,
};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

// status: 0-解除好友关系；1-同意请求；2-申请；3-拒绝

#[derive(Serialize, Queryable, Selectable, Debug, Insertable)]
#[diesel(table_name=friendships)]
// 开启编译期字段检查，主要检查字段类型、数量是否匹配，可选
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FriendShipDb {
    pub id: String,
    pub user_id: String,
    pub friend_id: String,
    pub status: String,
    pub apply_msg: Option<String>,
    pub source: Option<String>,
    pub create_time: chrono::NaiveDateTime,
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

// 创建好友申请
pub async fn create_friend_ship(pool: &Pool, new_friend: FriendShipDb) -> Result<User, InfraError> {
    // 我想你发出申请，需要获取我的用户信息发送给你。
    let user_id = new_friend.user_id.clone();

    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    conn.interact(move |conn| {
        diesel::insert_into(friendships::table)
            .values(&new_friend)
            .execute(conn)
    })
    .await
    .map_err(adapt_infra_error)?
    .map_err(adapt_infra_error)?;
    let user = conn
        .interact(move |conn| {
            users::table
                .filter(users::id.eq(user_id))
                .select(User::as_select())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(user)
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

#[derive(Debug, Deserialize)]
pub struct NewFriend {
    pub user_id: String,
    pub friend_id: String,
    pub status: String,
    pub remark: Option<String>,
    pub source: Option<String>,
}
pub async fn agree_apply(pool: &Pool, mut new_friend: NewFriend) -> Result<(), InfraError> {
    let _friend_ship = update_friend_ship(
        pool,
        new_friend.friend_id.clone(),
        new_friend.user_id.clone(),
        "1".to_string(),
    )
    .await?;
    let now = chrono::Local::now().naive_local();
    let friends = vec![
        FriendDb {
            id: nanoid!(),
            user_id: new_friend.user_id.clone(),
            friend_id: new_friend.friend_id.clone(),
            status: "1".to_string(),
            remark: new_friend.remark,
            source: new_friend.source.clone(),
            create_time: now,
            update_time: now,
        },
        FriendDb {
            id: nanoid!(),
            user_id: new_friend.friend_id,
            friend_id: new_friend.user_id,
            status: "1".to_string(),
            remark: None,
            source: new_friend.source,
            create_time: now,
            update_time: now,
        },
    ];

    let _friend_db = create_friend(pool, friends).await?;

    Ok(())
}

// 根据用户id以及申请状态查询对应的记录
pub async fn get_by_user_id_and_status(
    pool: &Pool,
    user_id: String,
    status: String,
) -> Result<Vec<User>, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    let users = conn
        .interact(|conn| {
            let friends_ids = friendships::table
                .filter(
                    friendships::friend_id
                        .eq(user_id)
                        .and(friendships::status.eq(status)),
                )
                .select(friendships::user_id);
            users::table
                .filter(users::id.eq_any(friends_ids))
                .load::<User>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(users)
}
