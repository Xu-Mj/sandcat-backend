-- drop table friends;-- This file should undo anything in `up.sql`

-- 1. 删除触发器
DROP TRIGGER IF EXISTS trg_update_interaction_score ON friend_interactions;

-- 2. 删除触发器函数
DROP FUNCTION IF EXISTS update_interaction_score();


-- 4. 删除索引
DROP INDEX IF EXISTS idx_friends_starred;
DROP INDEX IF EXISTS idx_friends_active;
DROP INDEX IF EXISTS idx_friend_interactions_score;
DROP INDEX IF EXISTS idx_friends_user_starred;
DROP INDEX IF EXISTS idx_friendships_user_status;
DROP INDEX IF EXISTS idx_friend_interactions_last_time;
DROP INDEX IF EXISTS idx_friend_interactions_user;
DROP INDEX IF EXISTS idx_friend_tags_user;
DROP INDEX IF EXISTS idx_friend_groups_user;
DROP INDEX IF EXISTS idx_friends_user_group;
DROP INDEX IF EXISTS idx_friends_friend;
DROP INDEX IF EXISTS idx_friends_user;
DROP INDEX IF EXISTS idx_friends_fs_id;
DROP INDEX IF EXISTS idx_friendships_status;
DROP INDEX IF EXISTS idx_friendships_friend;
DROP INDEX IF EXISTS idx_friendships_user;
DROP INDEX IF EXISTS idx_friendships_fs_id;

-- 5. 删除所有表（按照依赖关系的反序，先删除被依赖表，再删除依赖表）
-- 首先删除依赖friend表的关联表
DROP TABLE IF EXISTS friend_tag_relations;
DROP TABLE IF EXISTS friend_privacy_settings;
DROP TABLE IF EXISTS friend_interactions;
DROP TABLE IF EXISTS friend_notes;

-- 然后删除friend表

-- 删除不再有依赖的其他表
DROP TABLE IF EXISTS friend_tags;
DROP TABLE IF EXISTS friendships;
DROP TABLE IF EXISTS friends;
DROP TABLE IF EXISTS friend_groups;

-- 6. 最后删除枚举类型
DROP TYPE IF EXISTS friend_request_status;
