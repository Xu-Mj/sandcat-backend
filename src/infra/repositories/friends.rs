use crate::domain::model::user::User;
use crate::infra::db::schema::{friends, users};
use crate::infra::errors::{adapt_infra_error, InfraError};
use deadpool_diesel::postgres::Pool;
use diesel::upsert::excluded;
use diesel::{
    Associations, BoolExpressionMethods, ExpressionMethods, Insertable, JoinOnDsl, QueryDsl,
    Queryable, RunQueryDsl, Selectable, SelectableHelper,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Queryable, Selectable, Associations, Debug, Insertable)]
#[diesel(table_name = friends)]
#[diesel(belongs_to(User))]
// 开启编译期字段检查，主要检查字段类型、数量是否匹配，可选
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FriendDb {
    pub id: String,
    pub friendship_id: String,
    pub user_id: String,
    pub friend_id: String,
    // 0: delete; 1: friend; 2: blacklist
    pub status: String,
    pub remark: Option<String>,
    pub hello: Option<String>,
    pub source: Option<String>,
    pub create_time: chrono::NaiveDateTime,
    pub update_time: chrono::NaiveDateTime,
}

#[derive(Serialize, Queryable, Deserialize, Clone, Debug)]
pub struct FriendWithUser {
    pub id: String,
    pub friend_id: String,
    pub remark: Option<String>,
    pub hello: Option<String>,
    pub status: String,
    pub create_time: chrono::NaiveDateTime,
    pub update_time: chrono::NaiveDateTime,
    pub from: Option<String>,
    pub name: String,
    pub account: String,
    pub avatar: String,
    pub gender: String,
    pub age: i32,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub birthday: Option<chrono::NaiveDateTime>,
}

// 获取好友列表
pub async fn get_friend_list(
    pool: &Pool,
    user_id: String,
) -> Result<Vec<FriendWithUser>, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    let users = conn
        .interact(move |conn| {
            friends::table
                .inner_join(users::table.on(users::id.eq(friends::friend_id)))
                .filter(friends::user_id.eq(user_id.clone()))
                .filter(friends::status.eq("1"))
                .select((
                    friends::id,
                    friends::friend_id,
                    friends::remark,
                    friends::hello,
                    friends::status,
                    friends::create_time,
                    friends::update_time,
                    friends::source,
                    users::name,
                    users::account,
                    users::avatar,
                    users::gender,
                    users::age,
                    users::phone,
                    users::email,
                    users::address,
                    users::birthday,
                ))
                .load::<FriendWithUser>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(users)
}

pub async fn create_friend(pool: &Pool, friends: Vec<FriendDb>) -> Result<(), InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    let _ = conn
        .interact(move |conn| {
            diesel::insert_into(friends::table)
                .values(&friends)
                .on_conflict((friends::user_id, friends::friend_id))
                .do_update()
                .set((
                    friends::status.eq(excluded(friends::status)),
                    friends::remark.eq(excluded(friends::remark)),
                    friends::hello.eq(excluded(friends::hello)),
                    friends::source.eq(excluded(friends::source)),
                ))
                .execute(conn)
        })
        .await
        .map_err(|err| {
            tracing::error!("create user error: {:?}", err);
            adapt_infra_error(err)
        })?
        .map_err(|err| {
            tracing::error!("create user error: {:?}", err);
            adapt_infra_error(err)
        })?;
    Ok(())
}

// 更新好友信息，就是更新remark、status、
pub async fn update_remark(
    pool: &Pool,
    user_id: String,
    friend_id: String,
    remark: String,
) -> Result<FriendDb, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;

    let friend = conn
        .interact(|conn| {
            diesel::update(friends::table)
                .filter(
                    friends::user_id
                        .eq(user_id)
                        .and(friends::friend_id.eq(friend_id)),
                )
                .set((
                    friends::remark.eq(remark),
                    friends::update_time.eq(chrono::Local::now().naive_local()),
                ))
                .returning(FriendDb::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(friend)
}

// 更新好友信息，就是更新status、
pub async fn update_friend_status(
    pool: &Pool,
    user_id: String,
    friend_id: String,
    status: String,
) -> Result<FriendDb, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;

    let friend = conn
        .interact(|conn| {
            diesel::update(friends::table)
                .filter(
                    friends::user_id
                        .eq(user_id)
                        .and(friends::friend_id.eq(friend_id)),
                )
                .set((
                    friends::remark.eq(status),
                    friends::update_time.eq(chrono::Local::now().naive_local()),
                ))
                .returning(FriendDb::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(friend)
}
