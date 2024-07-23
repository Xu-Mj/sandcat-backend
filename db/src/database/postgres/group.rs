use async_trait::async_trait;
use sqlx::PgPool;

use abi::errors::Error;
use abi::message::{
    GetGroupAndMembersResp, GroupCreate, GroupInfo, GroupInvitation, GroupInviteNew, GroupMember,
    GroupUpdate,
};

use crate::database::group::GroupStoreRepo;

pub struct PostgresGroup {
    pool: PgPool,
}

impl PostgresGroup {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GroupStoreRepo for PostgresGroup {
    async fn get_group(&self, user_id: &str, group_id: &str) -> Result<GroupInfo, Error> {
        let group = sqlx::query_as(
            "SELECT * FROM groups g
                JOIN group_members gm ON g.id = gm.group_id
                WHERE g.id = $1 AND gm.user_id = $2",
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(group)
    }

    async fn get_group_and_members(
        &self,
        user_id: &str,
        group_id: &str,
    ) -> Result<GetGroupAndMembersResp, Error> {
        let group = sqlx::query_as(
            "SELECT * FROM groups g
                JOIN group_members gm ON g.id = gm.group_id
                WHERE g.id = $1 AND gm.user_id = $2",
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let members = sqlx::query_as(
            "SELECT gm.group_id, gm.role AS role, gm.joined_at, u.id AS user_id, u.name AS group_name,
                u.avatar AS avatar, u.age AS age, u.region AS region, u.gender AS gender,
                u.signature AS signature
                FROM group_members gm
                JOIN users u
                ON u.id = gm.user_id
                WHERE group_id = $1")
            .bind(group_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(GetGroupAndMembersResp::new(group, members))
    }

    async fn get_members(
        &self,
        user_id: &str,
        group_id: &str,
        mem_ids: Vec<String>,
    ) -> Result<Vec<GroupMember>, Error> {
        // check user belongs to the group
        let user_belongs_to_group: (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2)",
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if !user_belongs_to_group.0 {
            return Err(Error::NotFound);
        }

        let members =
            sqlx::query_as(
                "SELECT gm.group_id, gm.role AS role, gm.joined_at, u.id AS user_id, u.name AS group_name,
                    u.avatar AS avatar, u.age AS age, u.region AS region, u.gender AS gender,
                    u.signature AS signature
                    FROM group_members gm
                    JOIN users u
                    ON u.id = gm.user_id
                    WHERE group_id = $1 AND user_id = ANY($2)"
                )
                .bind(group_id)
                .bind(&mem_ids)
                .fetch_all(&self.pool)
                .await?;
        Ok(members)
    }

    async fn create_group_with_members(
        &self,
        group: &GroupCreate,
    ) -> Result<GroupInvitation, Error> {
        let now = chrono::Utc::now().timestamp_millis();
        let mut tx = self.pool.begin().await?;
        let mut invitation = GroupInvitation::default();
        // create group
        let info: GroupInfo = sqlx::query_as(
            "INSERT INTO groups
                (id, owner, name, avatar, create_time, update_time)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 RETURNING *",
        )
        .bind(&group.id)
        .bind(&group.owner)
        .bind(&group.group_name)
        .bind(&group.avatar)
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;
        invitation.info = Some(info);

        // create members
        // select user info by members id and then insert into group_members
        let members: Vec<GroupMember> =
            sqlx::query_as(
                "WITH inserted AS (
                    INSERT INTO group_members (user_id, group_id, group_name, role, joined_at)
                    SELECT u.id AS user_id, $1 AS group_id, u.name AS group_name,
                        CASE WHEN u.id = $4 THEN 'Owner'::group_role ELSE 'Member'::group_role END AS role,
                        $2 AS joined_at
                    FROM users AS u
                    WHERE u.id = ANY($3)
                    RETURNING user_id, group_id, role, joined_at
                )
                SELECT ins.group_id, ins.joined_at, usr.id AS user_id, usr.name AS group_name,
                        usr.avatar AS avatar, usr.age AS age, usr.region AS region, usr.gender AS gender,
                        usr.signature AS signature, ins.role AS role
                FROM inserted AS ins
                JOIN users AS usr ON ins.user_id = usr.id;
                ")
                .bind(&group.id)
                .bind(now)
                .bind(&group.members_id)
                .bind(&group.owner)
                .fetch_all(&mut *tx)
                .await?;
        invitation.members = members;
        tx.commit().await?;
        Ok(invitation)
    }

    async fn invite_new_members(&self, group: &GroupInviteNew) -> Result<(), Error> {
        let user_belongs_to_group: (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2)",
        )
        .bind(&group.group_id)
        .bind(&group.user_id)
        .fetch_one(&self.pool)
        .await?;

        if !user_belongs_to_group.0 {
            return Err(Error::NotFound);
        }

        // insert new members
        sqlx::query(
            "INSERT INTO group_members (user_id, group_id, group_name, joined_at, role)
                SELECT u.id, $1 as group_id, u.name AS group_name,  $2 AS joined_at, 'Member'::group_role AS role
                FROM users AS u
                WHERE u.id = ANY($3)
                RETURNING user_id, group_id, joined_at, role
                ")
            .bind(&group.group_id)
            .bind(chrono::Utc::now().timestamp_millis())
            .bind(&group.members)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_member(
        &self,
        group_id: &str,
        user_id: &str,
        mem_id: &str,
    ) -> Result<(), Error> {
        sqlx::query(
            "DELETE FROM group_members
            WHERE user_id = $1
            AND group_id = $2
            AND EXISTS (
                SELECT 1 FROM group_members
                WHERE group_id = $1
                AND user_id = $3
                AND (role = 'Admin' OR role = 'Owner'))",
        )
        .bind(mem_id)
        .bind(group_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_group_by_id(&self, group_id: &str) -> Result<GroupInfo, Error> {
        let info = sqlx::query_as("SELECT * FROM groups WHERE id = $1")
            .bind(group_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(info)
    }

    async fn query_group_members_id(&self, group_id: &str) -> Result<Vec<String>, Error> {
        let result: Vec<(String,)> =
            sqlx::query_as("SELECT user_id FROM group_members WHERE group_id = $1")
                .bind(group_id)
                .fetch_all(&self.pool)
                .await?;
        let result = result.into_iter().map(|(user_id,)| user_id).collect();
        Ok(result)
    }

    async fn query_group_members_by_group_id(
        &self,
        group_id: &str,
    ) -> Result<Vec<GroupMember>, Error> {
        let members = sqlx::query_as(
            "SELECT m.id, m.user_id, m.group_id, m.group_name, m.group_remark, m.joined_at
             FROM group_members
             WHERE group_id = $1",
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(members)
    }

    async fn update_group(&self, group: &GroupUpdate) -> Result<GroupInfo, Error> {
        let now = chrono::Utc::now().timestamp_millis();
        let group = sqlx::query_as(
            "UPDATE groups SET
             name = COALESCE(NULLIF($1, ''), name),
             avatar = COALESCE(NULLIF($2, ''), avatar),
             description = COALESCE(NULLIF($3, ''), description),
             announcement = COALESCE(NULLIF($4, ''), announcement),
             update_time = $5
             WHERE id = $6 RETURNING *",
        )
        .bind(&group.name)
        .bind(&group.avatar)
        .bind(&group.description)
        .bind(&group.announcement)
        .bind(now)
        .bind(&group.id)
        .fetch_one(&self.pool)
        .await?;
        Ok(group)
    }

    async fn exit_group(&self, user_id: &str, group_id: &str) -> Result<(), Error> {
        sqlx::query("DELETE FROM group_members WHERE user_id = $1 AND group_id = $2")
            .bind(user_id)
            .bind(group_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_group(&self, group_id: &str, owner: &str) -> Result<GroupInfo, Error> {
        // delete group and group members
        let mut tx = self.pool.begin().await?;
        let group = sqlx::query_as("DELETE FROM groups WHERE id = $1 and owner = $2 RETURNING *")
            .bind(group_id)
            .bind(owner)
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM group_members WHERE group_id = $1")
            .bind(group_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(group)
    }
}
