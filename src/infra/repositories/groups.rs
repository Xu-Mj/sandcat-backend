use deadpool_diesel::postgres::Pool;
use diesel::{ExpressionMethods, Insertable, QueryDsl, Queryable, RunQueryDsl, Selectable};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::domain::model::group_members::GroupMemberWithUser;
use crate::domain::model::msg::CreateGroup;
use crate::handlers::groups::GroupRequest;
use crate::infra::db::schema::groups;
use crate::infra::errors::{adapt_infra_error, InfraError};

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = groups)]
// 开启编译期字段检查，主要检查字段类型、数量是否匹配，可选
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GroupDb {
    pub id: String,
    pub owner: String,
    pub name: String,
    pub avatar: String,
    pub description: String,
    pub announcement: String,
    pub create_time: chrono::NaiveDateTime,
    pub update_time: chrono::NaiveDateTime,
}

impl From<GroupRequest> for GroupDb {
    fn from(value: GroupRequest) -> Self {
        GroupDb {
            id: nanoid!(),
            owner: value.owner,
            name: value.group_name,
            avatar: value.avatar,
            description: String::new(),
            announcement: String::new(),
            create_time: chrono::Local::now().naive_local(),
            update_time: chrono::Local::now().naive_local(),
        }
    }
}

/// create a new record
pub async fn create_group_with_members(
    pool: &PgPool,
    mut group: CreateGroup,
    members: Vec<String>,
) -> Result<CreateGroup, InfraError> {
    let mut tx = pool.begin().await?;
    // create group
    sqlx::query(
        "INSERT INTO groups
            (id, owner, name, avatar, description, announcement, create_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(&group.info.id)
    .bind(&group.info.owner)
    .bind(&group.info.name)
    .bind(&group.info.avatar)
    .bind(&group.info.description)
    .bind(&group.info.announcement)
    .bind(group.info.create_time)
    .bind(group.info.update_time)
    .execute(&mut *tx)
    .await?;
    // create members
    // select user info by members id and then insert into group_members

    let members: Vec<GroupMemberWithUser> =
        sqlx::query_as(
            "WITH inserted AS (
                    INSERT INTO group_members as t (user_id, group_id, group_name, delivered, joined_at)
                    SELECT u.id, $1 as group_id, u.name AS group_name,  $2 AS delivered, $3 AS joined_at
                    FROM users AS u
                    WHERE u.id = ANY($4)
                    RETURNING t.id, user_id, group_id, joined_at
                )
                SELECT ins.id, ins.group_id, ins.joined_at, usr.id AS user_id, usr.name AS group_name, usr.avatar AS avatar, usr.age AS age, usr.region AS region, usr.gender AS gender
                FROM inserted AS ins
                JOIN users AS usr ON ins.user_id = usr.id;
                ")
            .bind(&group.info.id)
            .bind(false)
            .bind(group.info.create_time)
            .bind(members)
            .fetch_all(&mut *tx)
            .await?;
    group.members = members;
    tx.commit().await?;
    Ok(group)
}

/// query group by group id
pub async fn get_group_by_id(pool: &Pool, group_id: String) -> Result<GroupDb, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    let group = conn
        .interact(move |conn| {
            groups::table
                .filter(groups::id.eq(&group_id))
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(group)
}
