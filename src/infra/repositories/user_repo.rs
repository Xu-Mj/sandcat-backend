use crate::domain::model::user::User;
use crate::infra::db::schema::users;
use crate::infra::errors::{adapt_infra_error, InfraError};
use deadpool_diesel::postgres::Pool;
use diesel::{
    BoolExpressionMethods, ExpressionMethods, Insertable, QueryDsl, RunQueryDsl, SelectableHelper, TextExpressionMethods,
};
use serde::Deserialize;

// #[derive(Serialize, Queryable, Selectable, Debug)]
// // 指定表明
// #[diesel(table_name=users)]
// // 开启编译期字段检查，主要检查字段类型、数量是否匹配，可选
// #[diesel(check_for_backend(diesel::pg::Pg))]
// pub struct UserDb {
//     pub id: String,
//     pub name: String,
//     pub account: String,
//     pub password: String,
//     pub avatar: String,
//     pub gender: String,
//     pub phone: Option<String>,
//     pub email: Option<String>,
//     pub address: Option<String>,
//     pub birthday: Option<chrono::NaiveDateTime>,
//     pub create_time: chrono::NaiveDateTime,
//     pub update_time: chrono::NaiveDateTime,
//     pub is_delete: bool,
// }

// 新建用户结构体
#[derive(Deserialize, Insertable, Default)]
// 指定表明
#[diesel(table_name = users)]
pub struct NewUserDb {
    pub id: String,
    pub name: String,
    pub account: String,
    pub password: String,
    pub avatar: String,
    pub gender: String,
    pub age: i32,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub birthday: Option<chrono::NaiveDateTime>,
}

// 新建
pub async fn insert(pool: &Pool, new_user: NewUserDb) -> Result<User, InfraError> {
    // 设置user_id
    // 从连接池获取链接, map_err的作用是转换抛出的错误
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    // 执行sql操作
    let result = conn
        .interact(|conn| {
            diesel::insert_into(users::table)
                .values(new_user)
                .returning(User::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(result)
    // Ok(User::from(result))
}

pub async fn get(pool: &Pool, id: String) -> Result<User, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let user = conn
        .interact(move |conn| {
            users::table
                .filter(users::id.eq(id).and(users::is_delete.eq(false)))
                .select(User::as_select())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    // tracing::debug!("get user by id: {:?}", &user);
    Ok(user)
}

pub async fn get_by_2id(
    pool: &Pool,
    user_id: String,
    friend_id: String,
) -> Result<(User, User), InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let cloned_user_id = user_id.clone();
    let mut users = conn
        .interact(move |conn| {
            users::table
                .filter(
                    users::id
                        .eq_any(vec![cloned_user_id, friend_id])
                        .and(users::is_delete.eq(false)),
                )
                .select(User::as_select())
                .get_results(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    let user1 = users.remove(0);
    // tracing::debug!("get user by id: {:?}", &user);
    Ok(if user1.id == user_id.clone() {
        (user1, users.remove(0))
    } else {
        (users.remove(0), user1)
    })
}

pub async fn search(
    pool: &Pool,
    user_id: String,
    pattern: String,
) -> Result<Vec<User>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let users = conn
        .interact(move |conn| {
            users::table
                .filter(
                    users::account
                        .eq(&pattern)
                        .or(users::name.like(&pattern))
                        .or(users::phone.eq(&pattern))
                        .and(users::is_delete.eq(false))
                        .and(users::id.ne(user_id)),
                )
                .select(User::as_select())
                .get_results(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    // tracing::debug!("get user by id: {:?}", &user);
    Ok(users)
}

pub async fn verify_pwd(pool: &Pool, account: String, pwd: String) -> Result<User, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let user = conn
        .interact(move |conn| {
            users::table
                .filter(
                    users::account.eq(account.clone()).or(users::phone
                        .eq(account.clone())
                        .or(users::email.eq(account))),
                )
                .select(User::as_select())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    if user.password == pwd {
        Ok(user)
    } else {
        Err(InfraError::NotFound)
    }
}
