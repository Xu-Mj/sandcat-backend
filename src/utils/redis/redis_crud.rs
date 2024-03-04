use crate::infra::errors::InfraError;
use redis::aio::Connection;
use redis::AsyncCommands;
use serde::Serialize;

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

pub async fn del(mut conn: Connection, key: String) -> Result<(), InfraError> {
    conn.del(key)
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    Ok(())
}
