use sqlx::PgPool;

use crate::domain::model::group_members::GroupMember;
use crate::infra::errors::InfraError;
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
