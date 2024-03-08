use redis::aio::Connection;
use redis::AsyncCommands;
use redis::Commands;
use serde::Serialize;

use crate::domain::model::group_members::GroupMemberWithUser;
use crate::domain::model::msg::CreateGroup;
use crate::infra::errors::InfraError;
use crate::infra::repositories::groups::GroupDb;

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

pub fn store_group(mut conn: redis::Connection, group: &CreateGroup) -> redis::RedisResult<()> {
    // Convert the group info to a JSON string
    let group_info_json = serde_json::to_string(&group.info).unwrap();

    // Store the group info in a hash with the ID as the key
    conn.hset("groups_info", &group.info.id, group_info_json)?;
    let key = format!("group_members:{}", &group.info.id);

    // Add each member to the set for the group by redis pipe
    let mut pipe = redis::pipe();
    for member in &group.members {
        let member_json = serde_json::to_string(&member).unwrap();
        pipe.hset(&key, &member.user_id, member_json);
    }
    pipe.query(&mut conn)?;

    Ok(())
}

pub fn get_members_id(
    conn: &mut redis::Connection,
    group_id: &str,
) -> redis::RedisResult<Vec<String>> {
    let key = format!("group_members:{}", group_id);
    let members: Vec<String> = conn.hkeys(&key).unwrap();
    Ok(members)
}
pub fn get_group_info(conn: &mut redis::Connection, group_id: &str) -> redis::RedisResult<GroupDb> {
    let group_info: String = conn.hget("groups_info", group_id)?;
    let group = serde_json::from_str(&group_info).unwrap();
    Ok(group)
}

pub fn get_group_members(
    conn: &mut redis::Connection,
    group_id: &str,
) -> redis::RedisResult<Vec<GroupMemberWithUser>> {
    let key = format!("group_members:{}", group_id);
    let members: Vec<String> = conn.smembers(&key)?;
    let members: Vec<GroupMemberWithUser> = members
        .into_iter()
        .map(|member| serde_json::from_str(&member).unwrap())
        .collect();
    Ok(members)
}

pub async fn del(mut conn: Connection, key: String) -> Result<(), InfraError> {
    conn.del(key)
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use redis::Client;

    use crate::config;

    use super::*;

    #[tokio::test]
    async fn test_set_string() {
        let redis = Client::open(config::config().await.redis_url()).expect("redis can't open");
        let mut conn = redis.get_connection().expect("redis can't get connection");
        let error = get_group_info(&mut conn, "123").unwrap_err();
        println!(
            "error kind:{:#?}; code:{:#?}; category:{:#?}; detail:{:#?}",
            error.kind(),
            error.code(),
            error.category(),
            error.detail(),
        )
    }
}
