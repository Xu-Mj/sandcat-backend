use crate::domain::model::user::UserView;
use redis::aio::Connection;
use redis::AsyncCommands;
use redis::Commands;
use serde::Serialize;

use crate::infra::errors::InfraError;

pub async fn set_string<T: Serialize>(
    mut conn: Connection,
    key: String,
    value: T,
) -> Result<String, InfraError> {
    let value = serde_json::to_string(&value)
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    // redis 还提供了json特性，底层也是通过serde_json::to_string来实现的，用不用都行

    let result = conn
        .set(key, value)
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    Ok(result)
}

pub fn store_user_views(
    mut conn: redis::Connection,
    members: &Vec<UserView>,
) -> redis::RedisResult<()> {
    let mut pipe = redis::pipe();
    for member in members {
        pipe.set(&member.id, serde_json::to_string(member).unwrap())
            .ignore();
    }
    pipe.query(&mut conn)?;
    Ok(())
}

#[allow(dead_code)]
pub fn get_user_view(
    mut conn: redis::Connection,
    user_id: &str,
) -> redis::RedisResult<Option<UserView>> {
    let serialized: Option<String> = conn.get(user_id)?;
    match serialized {
        Some(s) => {
            let user_view: UserView = serde_json::from_str(&s).unwrap();
            Ok(Some(user_view))
        }
        None => Ok(None),
    }
}
pub async fn del(mut conn: Connection, key: String) -> Result<(), InfraError> {
    conn.del(key)
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    Ok(())
}
