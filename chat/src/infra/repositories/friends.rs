use deadpool_diesel::postgres::Pool;
use diesel::upsert::excluded;
use diesel::ExpressionMethods;
use diesel::{BoolExpressionMethods, JoinOnDsl, QueryDsl, RunQueryDsl, SelectableHelper};

use crate::domain::model::friend_request_status::FriendStatus;
use crate::domain::model::friends::{FriendDb, FriendWithUser};
use crate::infra::db::schema::{friends, users};
use crate::infra::errors::{adapt_infra_error, InfraError};

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
                .filter(friends::status.eq(FriendStatus::Accepted))
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
            tracing::error!("create user errors: {:?}", err);
            adapt_infra_error(err)
        })?
        .map_err(|err| {
            tracing::error!("create user errors: {:?}", err);
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
