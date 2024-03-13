use diesel::{Insertable, Queryable, Selectable};
use nanoid::nanoid;
use redis::Connection;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{Error, FromRow, PgPool, Row};
use tracing::warn;

use crate::domain::model::group_members::GroupMemberWithUser;
use crate::domain::model::msg::GroupInvitation;
use crate::handlers::groups::GroupRequest;
use crate::infra::db::schema::groups;
use crate::infra::errors::InfraError;
use crate::infra::Validator;
use crate::utils::redis::redis_crud::{
    get_group_info, get_group_members, store_group_info, store_group_members,
};

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
        let now = chrono::Local::now().naive_local();
        GroupDb {
            id: nanoid!(),
            owner: value.owner,
            name: value.group_name,
            avatar: value.avatar,
            description: String::new(),
            announcement: String::new(),
            create_time: now,
            update_time: now,
        }
    }
}

impl Validator for GroupDb {
    fn validate(&self) -> Result<(), InfraError> {
        if self.name.is_empty() {
            return Err(InfraError::ValidateError);
        }
        Ok(())
    }
}

impl FromRow<'_, PgRow> for GroupDb {
    fn from_row(row: &'_ PgRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.get("id"),
            owner: row.get("owner"),
            name: row.get("name"),
            avatar: row.get("avatar"),
            description: row.get("description"),
            announcement: row.get("announcement"),
            create_time: row.get("create_time"),
            update_time: row.get("update_time"),
        })
    }
}
/// create a new record
pub async fn create_group_with_members(
    pool: &PgPool,
    mut group: GroupInvitation,
    members: Vec<String>,
) -> Result<GroupInvitation, InfraError> {
    group.info.validate()?;
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
                    SELECT u.id, $1 as group_id, u.name AS group_name,  CASE
                            WHEN u.id = $2 THEN true
                            ELSE false
                        END AS delivered, $3 AS joined_at
                    FROM users AS u
                    WHERE u.id = ANY($4)
                    RETURNING t.id, user_id, group_id, joined_at
                )
                SELECT ins.id, ins.group_id, ins.joined_at, usr.id AS user_id, usr.name AS group_name, usr.avatar AS avatar, usr.age AS age, usr.region AS region, usr.gender AS gender
                FROM inserted AS ins
                JOIN users AS usr ON ins.user_id = usr.id;
                ")
            .bind(&group.info.id)
            .bind(&group.info.owner)
            .bind(group.info.create_time)
            .bind(members)
            .fetch_all(&mut *tx)
            .await?;
    group.members = members;
    tx.commit().await?;
    Ok(group)
}

/// query group by group id
pub async fn get_group_by_id(
    redis: &mut Connection,
    pool: &PgPool,
    group_id: &str,
) -> Result<GroupDb, InfraError> {
    // query form redis first
    let info: GroupDb = match get_group_info(redis, group_id) {
        Ok(info) => info,
        Err(err) => {
            warn!(
                "query group info from redis failed: {:?}",
                InfraError::RedisError(err)
            );
            // if not found, query db
            let info = sqlx::query_as("SELECT * FROM groups WHERE id = $1")
                .bind(group_id)
                .fetch_one(pool)
                .await?;
            // store information to redis
            store_group_info(redis, &info).map_err(InfraError::RedisError)?;
            info
        }
    };
    Ok(info)
}

pub async fn query_group_invitation_by_user_id(
    pool: &PgPool,
    mut redis: Connection,
    user_id: String,
) -> Result<Vec<GroupInvitation>, InfraError> {
    // query group_members for groups id only by user id
    let groups: Vec<PgRow> =
        sqlx::query("SELECT group_id FROM group_members WHERE user_id = $1 and delivered = false")
            .bind(&user_id)
            .fetch_all(pool)
            .await?;
    let mut invitations = Vec::with_capacity(groups.len());
    for row in groups {
        let group_id = row.get(0);
        let invitation =
            query_group_info_and_members_by_group_id(pool, &mut redis, group_id).await?;
        invitations.push(invitation);
    }
    Ok(invitations)
}

pub async fn query_group_info_and_members_by_group_id(
    pool: &PgPool,
    redis: &mut Connection,
    group_id: String,
) -> Result<GroupInvitation, InfraError> {
    // query redis first
    let info = get_group_by_id(redis, pool, &group_id).await?;
    // query members
    let members: Vec<GroupMemberWithUser> = match get_group_members(redis, &group_id) {
        Ok(m) => m,
        Err(err) => {
            warn!(
                "query group members from redis failed: {:?}",
                InfraError::RedisError(err)
            );
            let members: Vec<GroupMemberWithUser>  = sqlx::query_as(
                "SELECT m.id, m.user_id, m.group_id, m.group_name, m.group_remark, m.joined_at u. FROM group_members WHERE group_id = $1",
            )
                .bind(&group_id)
                .fetch_all(pool)
                .await?;
            store_group_members(redis, &info.id, &members).map_err(InfraError::RedisError)?;
            members
        }
    };
    let group = GroupInvitation { info, members };
    Ok(group)
}

/// update group
pub async fn update_group(pool: &PgPool, group: &GroupDb) -> Result<GroupDb, InfraError> {
    let now = chrono::Local::now().naive_local();
    let group: GroupDb = sqlx::query_as(
        "UPDATE groups SET
         name = COALESCE(NULLIF($1, ''), name),
         avatar = COALESCE(NULLIF($2, ''), avatar),
         description = COALESCE(NULLIF($3, ''), description),
         announcement = COALESCE(NULLIF($4, ''), announcement),
         update_time = $5)
         WHERE id = $6",
    )
    .bind(&group.name)
    .bind(&group.avatar)
    .bind(&group.description)
    .bind(&group.announcement)
    .bind(now)
    .bind(&group.id)
    .fetch_one(pool)
    .await?;
    Ok(group)
}

pub async fn delete_group(
    pool: &PgPool,
    group_id: &str,
    owner: &str,
) -> Result<GroupDb, InfraError> {
    let group: GroupDb =
        sqlx::query_as("DELETE FROM groups WHERE id = $1 and owner = $2 RETURNING *")
            .bind(group_id)
            .bind(owner)
            .fetch_one(pool)
            .await?;
    Ok(group)
}
