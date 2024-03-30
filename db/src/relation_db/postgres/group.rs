use abi::config::Config;
use async_trait::async_trait;
use sqlx::PgPool;

use abi::errors::Error;
use abi::message::{GroupCreate, GroupInfo, GroupInvitation, GroupMember};

use crate::relation_db::group::GroupStoreRepo;

pub struct PostgresGroup {
    pool: PgPool,
}

impl PostgresGroup {
    #[allow(dead_code)]
    pub async fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn from_config(config: &Config) -> Self {
        let pool = PgPool::connect(&config.db.postgres.url()).await.unwrap();

        Self { pool }
    }
}

#[async_trait]
impl GroupStoreRepo for PostgresGroup {
    async fn create_group_with_members(
        &self,
        group: GroupCreate,
    ) -> Result<GroupInvitation, Error> {
        let now = chrono::Local::now().timestamp_millis();
        let mut tx = self.pool.begin().await?;
        let mut invitation = GroupInvitation::default();
        // create group
        let info: GroupInfo = sqlx::query_as(
            "INSERT INTO groups
            (id, owner, name, avatar, member_count)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&group.id)
        .bind(&group.owner)
        .bind(&group.group_name)
        .bind(&group.avatar)
        .bind(group.members_id.len() as i32)
        .fetch_one(&mut *tx)
        .await?;
        invitation.info = Some(info);
        // create members
        // select user info by members id and then insert into group_members

        let members: Vec<GroupMember> =
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
                .bind(&group.id)
                .bind(&group.owner)
                .bind(now)
                .bind(group.members_id)
                .fetch_all(&mut *tx)
                .await?;
        invitation.members = members;
        tx.commit().await?;
        Ok(invitation)
    }

    async fn get_group_by_id(&self, _group_id: &str) -> Result<GroupInfo, Error> {
        todo!()
    }
}
