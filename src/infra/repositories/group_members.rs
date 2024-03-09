use sqlx::PgPool;
use tracing::warn;

use crate::domain::model::group_members::GroupMember;
use crate::infra::errors::InfraError;
use crate::utils::redis::redis_crud::get_members_id;

#[allow(dead_code)]

pub async fn query_group_invitation_by_user_id(
    pool: &PgPool,
    user_id: String,
) -> Result<Vec<GroupMember>, InfraError> {
    let result: Vec<GroupMember> =
        sqlx::query_as(
        "SELECT id, user_id, group_id, group_name, group_remark, delivered, joined_at FROM group_members WHERE user_id = $1 AND delivered = false"
        ).bind(&user_id)
        .fetch_all(pool)
        .await?;
    Ok(result)
}

#[allow(dead_code)]
pub async fn group_invitation_delivered_by_user_id(
    pool: &PgPool,
    user_id: String,
) -> Result<GroupMember, InfraError> {
    let result: GroupMember = sqlx::query_as(
        "UPDATE group_members SET delivered = true WHERE user_id = $1 AND delivered = false",
    )
    .bind(&user_id)
    .fetch_one(pool)
    .await?;
    Ok(result)
}

pub async fn query_group_members_id(
    redis: &mut redis::Connection,
    pool: &PgPool,
    group_id: String,
) -> Result<Vec<String>, InfraError> {
    // query redis first
    let result: Vec<String> = match get_members_id(redis, &group_id) {
        Ok(list) => list,
        Err(err) => {
            warn!(
                "query group info from redis failed: {:?}",
                InfraError::RedisError(err)
            );
            // query from db
            let result: Vec<(String,)> =
                sqlx::query_as("SELECT user_id FROM group_members WHERE group_id = $1")
                    .bind(&group_id)
                    .fetch_all(pool)
                    .await?;
            result.into_iter().map(|(user_id,)| user_id).collect()
        }
    };
    Ok(result)
}
