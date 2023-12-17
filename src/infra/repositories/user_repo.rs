use crate::domain::model::user::User;
use crate::infra::db::schema::users;
use crate::infra::errors::{adapt_infra_error, InfraError};
use deadpool_diesel::postgres::Pool;
use diesel::{
    ExpressionMethods, Insertable, QueryDsl, Queryable, RunQueryDsl, Selectable, SelectableHelper,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Queryable, Selectable, Debug)]
// 指定表明
#[diesel(table_name=users)]
// 开启编译期字段检查，主要检查字段类型、数量是否匹配，可选
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserDb {
    pub id: i32,
    pub name: String,
    pub account: String,
    pub avatar: String,
    pub gender: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub birthday: Option<chrono::NaiveDateTime>,
    pub create_time: chrono::NaiveDateTime,
    pub update_time: chrono::NaiveDateTime,
    pub is_delete: bool,
}

// 新建用户结构体
#[derive(Deserialize, Insertable)]
// 指定表明
#[diesel(table_name=users)]
pub struct NewUserDb {
    pub name: String,
    pub account: String,
    pub avatar: String,
    pub gender: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub birthday: Option<chrono::NaiveDateTime>,
}

// 新建
pub async fn insert(pool: &Pool, new_user: NewUserDb) -> Result<User, InfraError> {
    // 从连接池获取链接, map_err的作用是转换抛出的错误
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    // 执行sql操作
    let result = conn
        .interact(|conn| {
            diesel::insert_into(users::table)
                .values(new_user)
                .returning(UserDb::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(User::from(result))
}

pub async fn get(pool: &Pool, id: i32) -> Result<User, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let user = conn
        .interact(move |conn| {
            users::table
                .filter(users::id.eq(id))
                .select(UserDb::as_select())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    tracing::debug!("get user by id: {:?}", &user);
    Ok(convert_user_db2user(user))
}

pub fn convert_user_db2user(user_db: UserDb) -> User {
    User {
        id: user_db.id,
        name: user_db.name,
        avatar: user_db.avatar,
        account: user_db.account,
        gender: user_db.gender,
        phone: user_db.phone,
        email: user_db.email,
        address: user_db.address,
        birthday: user_db.birthday,
        create_time: user_db.create_time,
        update_time: user_db.update_time,
    }
}
