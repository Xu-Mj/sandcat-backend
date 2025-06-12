use async_trait::async_trait;
use serde_json::json;
use sqlx::PgPool;

use abi::errors::{Error, Result};
use abi::message::{
    CreateAnnouncementRequest, CreateGroupCategoryRequest, CreatePollRequest,
    GetGroupAndMembersResp, GroupAnnouncement, GroupCategory, GroupCreate, GroupFile,
    GroupFileUploadRequest, GroupInfo, GroupInvitation, GroupInviteNew, GroupMember,
    GroupMuteRecord, GroupPoll, GroupUpdate, MuteGroupMemberRequest, UpdateGroupRequest,
    VotePollRequest,
};

use crate::group::GroupStoreRepo;

#[derive(Debug)]
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
    async fn get_group(&self, user_id: &str, group_id: &str) -> Result<GroupInfo> {
        let group = sqlx::query_as(
            "SELECT g.* FROM groups g
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
    ) -> Result<GetGroupAndMembersResp> {
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
            "SELECT gm.group_id, gm.role, gm.joined_at, u.id AS user_id, u.name AS group_name,
             u.avatar AS avatar, u.age AS age, u.region AS region, u.gender AS gender,
             u.signature AS signature, gm.is_muted, gm.notification_level,
             gm.last_read_time, gm.display_order, gm.joined_by
             FROM group_members gm
             JOIN users u ON u.id = gm.user_id
             WHERE group_id = $1",
        )
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
    ) -> Result<Vec<GroupMember>> {
        // Check user belongs to the group
        let user_belongs_to_group: (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2)",
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if !user_belongs_to_group.0 {
            return Err(Error::not_found());
        }

        let members = sqlx::query_as(
            "SELECT gm.group_id, gm.role, gm.joined_at, u.id AS user_id, u.name AS group_name,
             u.avatar AS avatar, u.age AS age, u.region AS region, u.gender AS gender,
             u.signature AS signature, gm.is_muted, gm.notification_level,
             gm.last_read_time, gm.display_order, gm.joined_by
             FROM group_members gm
             JOIN users u ON u.id = gm.user_id
             WHERE group_id = $1 AND user_id = ANY($2)",
        )
        .bind(group_id)
        .bind(&mem_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(members)
    }

    async fn create_group_with_members(&self, group: &GroupCreate) -> Result<GroupInvitation> {
        let now = chrono::Utc::now().timestamp_millis();
        let mut tx = self.pool.begin().await?;
        let mut invitation = GroupInvitation::default();

        // Create group with default values for new fields
        let info: GroupInfo = sqlx::query_as(
            "INSERT INTO groups
             (id, owner, name, avatar, description, announcement, create_time, update_time,
              max_members, is_public, join_approval_required, mute_all, only_admin_post)
             VALUES ($1, $2, $3, $4, '', '', $5, $5, 500, true, false, false, false)
             RETURNING *",
        )
        .bind(&group.id)
        .bind(&group.owner)
        .bind(&group.group_name)
        .bind(&group.avatar)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;
        invitation.info = Some(info);

        // Create members with default values for new fields
        let members: Vec<GroupMember> = sqlx::query_as(
            "WITH inserted AS (
                INSERT INTO group_members
                (user_id, group_id, group_name, role, joined_at, is_muted, notification_level, last_read_time, display_order, joined_by)
                SELECT u.id AS user_id, $1 AS group_id, u.name AS group_name,
                    CASE WHEN u.id = $4 THEN 'Owner'::group_role ELSE 'Member'::group_role END AS role,
                    $2 AS joined_at, false AS is_muted, 'all' AS notification_level, $2 AS last_read_time, 0 AS display_order, $4 AS joined_by
                FROM users AS u
                WHERE u.id = ANY($3)
                RETURNING user_id, group_id, role, joined_at, is_muted, notification_level, last_read_time, display_order, joined_by
            )
            SELECT ins.group_id, ins.joined_at, usr.id AS user_id, usr.name AS group_name,
                   usr.avatar AS avatar, usr.age AS age, usr.region AS region, usr.gender AS gender,
                   usr.signature AS signature, ins.role, ins.is_muted, ins.notification_level,
                   ins.last_read_time, ins.display_order, ins.joined_by
            FROM inserted AS ins
            JOIN users AS usr ON ins.user_id = usr.id;"
        )
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

    async fn invite_new_members(&self, group: &GroupInviteNew) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        // Check user belongs to the group and has proper permission
        let user_has_permission: (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM group_members gm
                JOIN groups g ON gm.group_id = g.id
                WHERE g.id = $1 AND gm.user_id = $2 AND
                (gm.role::text <= g.invite_permission::text)
            )",
        )
        .bind(&group.group_id)
        .bind(&group.user_id)
        .fetch_one(&self.pool)
        .await?;

        if !user_has_permission.0 {
            return Err(Error::unauthorized_with_details(
                "user doesn't have permission to invite",
            ));
        }

        // Insert new members
        sqlx::query(
            "INSERT INTO group_members
             (user_id, group_id, group_name, joined_at, role, is_muted, notification_level, last_read_time, display_order, joined_by)
             SELECT u.id, $1 as group_id, u.name AS group_name, $2 AS joined_at,
                'Member'::group_role AS role, false AS is_muted, 'all' AS notification_level,
                $2 AS last_read_time, 0 AS display_order, $3 AS joined_by
             FROM users AS u
             WHERE u.id = ANY($4)
             ON CONFLICT (group_id, user_id) DO NOTHING"
        )
        .bind(&group.group_id)
        .bind(now)
        .bind(&group.user_id) // Inviter becomes joined_by
        .bind(&group.members)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn remove_member(&self, group_id: &str, user_id: &str, mem_ids: &[String]) -> Result<()> {
        sqlx::query(
            "DELETE FROM group_members
             WHERE user_id = ANY($1)
             AND group_id = $2
             AND EXISTS (
                SELECT 1 FROM group_members
                WHERE group_id = $2
                AND user_id = $3
                AND (role = 'Admin' OR role = 'Owner')
             )",
        )
        .bind(mem_ids)
        .bind(group_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_group_by_id(&self, group_id: &str) -> Result<GroupInfo> {
        let info = sqlx::query_as("SELECT * FROM groups WHERE id = $1")
            .bind(group_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(info)
    }

    async fn query_group_members_id(&self, group_id: &str) -> Result<Vec<String>> {
        let result: Vec<(String,)> =
            sqlx::query_as("SELECT user_id FROM group_members WHERE group_id = $1")
                .bind(group_id)
                .fetch_all(&self.pool)
                .await?;

        let result = result.into_iter().map(|(user_id,)| user_id).collect();
        Ok(result)
    }

    async fn query_group_members_by_group_id(&self, group_id: &str) -> Result<Vec<GroupMember>> {
        let members = sqlx::query_as(
            "SELECT gm.group_id, gm.user_id, gm.group_name, gm.role, gm.joined_at,
             gm.is_muted, gm.notification_level, gm.last_read_time, gm.display_order, gm.joined_by,
             u.avatar, u.age, u.gender, u.region, u.signature
             FROM group_members gm
             JOIN users u ON gm.user_id = u.id
             WHERE gm.group_id = $1",
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(members)
    }

    async fn update_group(&self, group: &GroupUpdate) -> Result<GroupInfo> {
        let now = chrono::Utc::now().timestamp_millis();
        let group = sqlx::query_as(
            "UPDATE groups SET
             name = COALESCE(NULLIF($1, ''), name),
             avatar = COALESCE(NULLIF($2, ''), avatar),
             description = COALESCE(NULLIF($3, ''), description),
             announcement = COALESCE(NULLIF($4, ''), announcement),
             update_time = $5
             WHERE id = $6
             RETURNING *",
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

    async fn exit_group(&self, user_id: &str, group_id: &str) -> Result<()> {
        // Check if user is the owner - can't exit if owner
        let is_owner: (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM group_members
                WHERE user_id = $1 AND group_id = $2 AND role = 'Owner'
            )",
        )
        .bind(user_id)
        .bind(group_id)
        .fetch_one(&self.pool)
        .await?;

        if is_owner.0 {
            return Err(Error::unauthorized_with_details(
                "Owner cannot exit group. Transfer ownership first",
            ));
        }

        sqlx::query("DELETE FROM group_members WHERE user_id = $1 AND group_id = $2")
            .bind(user_id)
            .bind(group_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_group(&self, group_id: &str, owner: &str) -> Result<GroupInfo> {
        let mut tx = self.pool.begin().await?;

        let group = sqlx::query_as(
            "DELETE FROM groups
             WHERE id = $1 AND owner = $2
             RETURNING *",
        )
        .bind(group_id)
        .bind(owner)
        .fetch_one(&mut *tx)
        .await?;

        // Other tables will be deleted via CASCADE
        tx.commit().await?;
        Ok(group)
    }

    // 扩展方法实现
    async fn update_group_settings(&self, req: &UpdateGroupRequest) -> Result<GroupInfo> {
        let now = chrono::Utc::now().timestamp_millis();

        let group = sqlx::query_as(
            "UPDATE groups SET
             name = COALESCE(NULLIF($1, ''), name),
             avatar = COALESCE(NULLIF($2, ''), avatar),
             description = COALESCE(NULLIF($3, ''), description),
             announcement = COALESCE(NULLIF($4, ''), announcement),
             join_approval_required = $5,
             mute_all = $6,
             only_admin_post = $7,
             invite_permission = $8::group_role,
             category = COALESCE(NULLIF($9, ''), category),
             tags = $10,
             update_time = $11
             WHERE id = $12
             RETURNING *",
        )
        .bind(&req.name)
        .bind(&req.avatar)
        .bind(&req.description)
        .bind(&req.announcement)
        .bind(req.join_approval_required)
        .bind(req.mute_all)
        .bind(req.only_admin_post)
        .bind(req.invite_permission)
        .bind(&req.category)
        .bind(&req.tags)
        .bind(now)
        .bind(&req.group_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(group)
    }

    async fn update_member_settings(
        &self,
        group_id: &str,
        user_id: &str,
        notification_level: &str,
        display_order: i32,
    ) -> Result<GroupMember> {
        let member = sqlx::query_as(
            "UPDATE group_members SET
             notification_level = $1,
             display_order = $2
             WHERE group_id = $3 AND user_id = $4
             RETURNING *",
        )
        .bind(notification_level)
        .bind(display_order)
        .bind(group_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(member)
    }

    async fn create_group_category(
        &self,
        req: &CreateGroupCategoryRequest,
    ) -> Result<GroupCategory> {
        let now = chrono::Utc::now().timestamp_millis();
        let id = nanoid::nanoid!();

        let category = sqlx::query_as(
            "INSERT INTO group_categories
             (id, name, description, display_order, create_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $5)
             RETURNING *",
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.display_order)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(category)
    }

    async fn update_group_category(&self, category: &GroupCategory) -> Result<GroupCategory> {
        let now = chrono::Utc::now().timestamp_millis();

        let updated = sqlx::query_as(
            "UPDATE group_categories SET
             name = $1,
             description = $2,
             display_order = $3,
             update_time = $4
             WHERE id = $5
             RETURNING *",
        )
        .bind(&category.name)
        .bind(&category.description)
        .bind(category.display_order)
        .bind(now)
        .bind(&category.id)
        .fetch_one(&self.pool)
        .await?;

        Ok(updated)
    }

    async fn delete_group_category(&self, id: &str) -> Result<()> {
        // Set category to NULL for any groups using this category
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE groups SET category = NULL WHERE category = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM group_categories WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn get_group_categories(&self) -> Result<Vec<GroupCategory>> {
        let categories = sqlx::query_as("SELECT * FROM group_categories ORDER BY display_order")
            .fetch_all(&self.pool)
            .await?;

        Ok(categories)
    }

    async fn upload_group_file(&self, req: &GroupFileUploadRequest) -> Result<GroupFile> {
        let now = chrono::Utc::now().timestamp_millis();
        let id = nanoid::nanoid!();

        // Save file to storage - separate file service handling

        // Record in database
        let file = sqlx::query_as(
            "INSERT INTO group_files
             (id, group_id, uploader_id, file_name, file_url, file_size, file_type, upload_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING *",
        )
        .bind(&id)
        .bind(&req.group_id)
        .bind(&req.uploader_id)
        .bind(&req.file_name)
        .bind(&req.file_url) // URL should be generated by file service
        .bind(req.file_size)
        .bind(&req.file_type)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(file)
    }

    async fn delete_group_file(&self, file_id: &str, user_id: &str) -> Result<()> {
        // Check permission
        let can_delete: (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM group_files f
                JOIN group_members m ON f.group_id = m.group_id AND m.user_id = $2
                WHERE f.id = $1 AND (m.role = 'Owner' OR m.role = 'Admin' OR f.uploader_id = $2)
            )",
        )
        .bind(file_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if !can_delete.0 {
            return Err(Error::unauthorized_with_details(
                "No permission to delete this file",
            ));
        }

        // Delete file record
        sqlx::query("DELETE FROM group_files WHERE id = $1")
            .bind(file_id)
            .execute(&self.pool)
            .await?;

        // Delete actual file via file service

        Ok(())
    }

    async fn get_group_files(&self, group_id: &str) -> Result<Vec<GroupFile>> {
        let files = sqlx::query_as(
            "SELECT * FROM group_files
             WHERE group_id = $1
             ORDER BY upload_time DESC",
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(files)
    }

    async fn update_file_pin_status(&self, file_id: &str, is_pinned: bool) -> Result<GroupFile> {
        let file = sqlx::query_as(
            "UPDATE group_files
             SET is_pinned = $1
             WHERE id = $2
             RETURNING *",
        )
        .bind(is_pinned)
        .bind(file_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(file)
    }

    async fn create_poll(&self, req: &CreatePollRequest) -> Result<GroupPoll> {
        let now = chrono::Utc::now().timestamp_millis();
        let id = nanoid::nanoid!();

        // Create options JSON
        let options_json = serde_json::to_value(
            req.options
                .iter()
                .map(|opt| {
                    json!({
                        "id": nanoid::nanoid!(),
                        "text": opt,
                        "vote_count": 0,
                        "voter_ids": []
                    })
                })
                .collect::<Vec<_>>(),
        )?;

        let poll = sqlx::query_as(
            "INSERT INTO group_polls
             (id, group_id, creator_id, title, options, is_multiple, is_anonymous, created_at, expires_at, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active')
             RETURNING *"
        )
        .bind(&id)
        .bind(&req.group_id)
        .bind(&req.creator_id)
        .bind(&req.title)
        .bind(options_json)
        .bind(req.is_multiple)
        .bind(req.is_anonymous)
        .bind(now)
        .bind(req.expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(poll)
    }

    async fn vote_poll(&self, req: &VotePollRequest) -> Result<GroupPoll> {
        // 获取当前投票
        let poll: GroupPoll = sqlx::query_as("SELECT * FROM group_polls WHERE id = $1")
            .bind(&req.poll_id)
            .fetch_one(&self.pool)
            .await?;

        if poll.status != "active" {
            return Err(Error::unauthorized_with_details("Poll is not active"));
        }

        // 直接处理 poll.options，它已经是反序列化后的结构
        let mut options_array = poll.options.clone();

        // 更新投票计数
        for option in &mut options_array {
            let option_id = option.id.clone();

            if req.option_ids.contains(&option_id) {
                // 添加投票者
                if !poll.is_anonymous {
                    let mut voter_ids = option.voter_ids.clone();

                    if !voter_ids.contains(&req.user_id) {
                        voter_ids.push(req.user_id.clone());
                    }

                    option.voter_ids = voter_ids;
                }

                // 更新计数
                let count = option.vote_count;

                option.vote_count = count + 1;
            }
        }

        // 保存更新后的选项
        let updated_options = serde_json::to_string(&options_array)?;

        let updated_poll = sqlx::query_as(
            "UPDATE group_polls SET
             options = $1,
             results = $1
             WHERE id = $2
             RETURNING *",
        )
        .bind(updated_options)
        .bind(&req.poll_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(updated_poll)
    }

    async fn close_poll(&self, poll_id: &str, user_id: &str) -> Result<GroupPoll> {
        // Check permission
        let can_close: (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM group_polls p
                JOIN group_members m ON p.group_id = m.group_id
                WHERE p.id = $1 AND m.user_id = $2
                AND (m.role = 'Admin' OR m.role = 'Owner' OR p.creator_id = $2)
            )",
        )
        .bind(poll_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if !can_close.0 {
            return Err(Error::unauthorized_with_details(
                "No permission to close this poll",
            ));
        }

        let poll = sqlx::query_as(
            "UPDATE group_polls SET
             status = 'closed'
             WHERE id = $1
             RETURNING *",
        )
        .bind(poll_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(poll)
    }

    async fn get_group_polls(&self, group_id: &str) -> Result<Vec<GroupPoll>> {
        let polls = sqlx::query_as(
            "SELECT * FROM group_polls
             WHERE group_id = $1
             ORDER BY created_at DESC",
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(polls)
    }

    async fn get_poll_details(&self, poll_id: &str) -> Result<GroupPoll> {
        let poll = sqlx::query_as("SELECT * FROM group_polls WHERE id = $1")
            .bind(poll_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(poll)
    }

    async fn mute_group_member(&self, req: &MuteGroupMemberRequest) -> Result<GroupMuteRecord> {
        // Check permission
        let has_permission: (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM group_members
                WHERE group_id = $1 AND user_id = $2 AND (role = 'Admin' OR role = 'Owner')
            )",
        )
        .bind(&req.group_id)
        .bind(&req.operator_id)
        .fetch_one(&self.pool)
        .await?;

        if !has_permission.0 {
            return Err(Error::unauthorized_with_details(
                "No permission to mute members",
            ));
        }

        // Can't mute higher roles
        let target_role: (String,) = sqlx::query_as(
            "SELECT role::text FROM group_members
             WHERE group_id = $1 AND user_id = $2",
        )
        .bind(&req.group_id)
        .bind(&req.user_id)
        .fetch_one(&self.pool)
        .await?;

        let operator_role: (String,) = sqlx::query_as(
            "SELECT role::text FROM group_members
             WHERE group_id = $1 AND user_id = $2",
        )
        .bind(&req.group_id)
        .bind(&req.operator_id)
        .fetch_one(&self.pool)
        .await?;

        if target_role.0 <= operator_role.0 && target_role.0 != "Member" {
            return Err(Error::unauthorized_with_details(
                "Cannot mute higher or equal role",
            ));
        }

        // Create mute record
        let now = chrono::Utc::now().timestamp_millis();
        let id = nanoid::nanoid!();

        let record = sqlx::query_as(
            "INSERT INTO group_mute_records
             (id, group_id, user_id, operator_id, mute_until, reason, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *",
        )
        .bind(&id)
        .bind(&req.group_id)
        .bind(&req.user_id)
        .bind(&req.operator_id)
        .bind(req.mute_until)
        .bind(&req.reason)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        // Update member status
        sqlx::query(
            "UPDATE group_members SET
             is_muted = true
             WHERE group_id = $1 AND user_id = $2",
        )
        .bind(&req.group_id)
        .bind(&req.user_id)
        .execute(&self.pool)
        .await?;

        Ok(record)
    }

    async fn unmute_group_member(
        &self,
        group_id: &str,
        user_id: &str,
        operator_id: &str,
    ) -> Result<()> {
        // Check permission
        let has_permission: (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM group_members
                WHERE group_id = $1 AND user_id = $2 AND (role = 'Admin' OR role = 'Owner')
            )",
        )
        .bind(group_id)
        .bind(operator_id)
        .fetch_one(&self.pool)
        .await?;

        if !has_permission.0 {
            return Err(Error::unauthorized_with_details(
                "No permission to unmute members",
            ));
        }

        // Update mute records to expired
        sqlx::query(
            "UPDATE group_mute_records SET
             mute_until = extract(epoch from now()) * 1000
             WHERE group_id = $1 AND user_id = $2 AND mute_until > extract(epoch from now()) * 1000"
        )
        .bind(group_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        // Update member status
        sqlx::query(
            "UPDATE group_members SET
             is_muted = false
             WHERE group_id = $1 AND user_id = $2",
        )
        .bind(group_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_group_muted_members(&self, group_id: &str) -> Result<Vec<GroupMuteRecord>> {
        let records = sqlx::query_as(
            "SELECT * FROM group_mute_records
             WHERE group_id = $1 AND mute_until > extract(epoch from now()) * 1000
             ORDER BY created_at DESC",
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    async fn create_announcement(
        &self,
        req: &CreateAnnouncementRequest,
    ) -> Result<GroupAnnouncement> {
        // Check permission
        let has_permission: (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM group_members
                WHERE group_id = $1 AND user_id = $2 AND (role = 'Admin' OR role = 'Owner')
            )",
        )
        .bind(&req.group_id)
        .bind(&req.creator_id)
        .fetch_one(&self.pool)
        .await?;

        if !has_permission.0 {
            return Err(Error::unauthorized_with_details(
                "No permission to create announcements",
            ));
        }

        let now = chrono::Utc::now().timestamp_millis();
        let id = nanoid::nanoid!();

        let announcement = sqlx::query_as(
            "INSERT INTO group_announcements
             (id, group_id, creator_id, content, created_at, is_pinned)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *",
        )
        .bind(&id)
        .bind(&req.group_id)
        .bind(&req.creator_id)
        .bind(&req.content)
        .bind(now)
        .bind(req.is_pinned)
        .fetch_one(&self.pool)
        .await?;

        Ok(announcement)
    }

    async fn delete_announcement(&self, id: &str, user_id: &str) -> Result<()> {
        // Check permission
        let has_permission: (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM group_announcements a
                JOIN group_members m ON a.group_id = m.group_id
                WHERE a.id = $1 AND m.user_id = $2
                AND (m.role = 'Admin' OR m.role = 'Owner' OR a.creator_id = $2)
            )",
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if !has_permission.0 {
            return Err(Error::unauthorized_with_details(
                "No permission to delete this announcement",
            ));
        }

        sqlx::query("DELETE FROM group_announcements WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_group_announcements(&self, group_id: &str) -> Result<Vec<GroupAnnouncement>> {
        let announcements = sqlx::query_as(
            "SELECT * FROM group_announcements
             WHERE group_id = $1
             ORDER BY is_pinned DESC, created_at DESC",
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(announcements)
    }

    async fn pin_announcement(&self, id: &str, is_pinned: bool) -> Result<GroupAnnouncement> {
        let announcement = sqlx::query_as(
            "UPDATE group_announcements
             SET is_pinned = $1
             WHERE id = $2
             RETURNING *",
        )
        .bind(is_pinned)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(announcement)
    }

    async fn change_member_role(
        &self,
        group_id: &str,
        user_id: &str,
        target_id: &str,
        new_role: &str,
    ) -> Result<GroupMember> {
        // Check operator permission
        let operator_role: (String,) = sqlx::query_as(
            "SELECT role::text FROM group_members
             WHERE group_id = $1 AND user_id = $2",
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if operator_role.0 != "Owner" && (operator_role.0 != "Admin" || new_role == "Owner") {
            return Err(Error::unauthorized_with_details(
                "Insufficient permissions for role change",
            ));
        }

        // Update member role
        let member = sqlx::query_as(
            "UPDATE group_members
             SET role = $1::group_role
             WHERE group_id = $2 AND user_id = $3
             RETURNING *",
        )
        .bind(new_role)
        .bind(group_id)
        .bind(target_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(member)
    }
}
