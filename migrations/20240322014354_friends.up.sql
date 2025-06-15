-- 1. 创建好友请求状态枚举（增加额外状态）
CREATE TYPE friend_request_status AS ENUM(
    'Pending',   -- 待处理
    'Accepted',  -- 已接受
    'Rejected',  -- 已拒绝
    'Blacked',   -- 已拉黑
    'Deleted',   -- 已删除
    'Muted',     -- 消息免打扰
    'Hidden'     -- 隐藏联系人
);

-- 2. 创建好友关系表
CREATE TABLE friendships (
    id VARCHAR PRIMARY KEY,
    user_id VARCHAR NOT NULL,
    friend_id VARCHAR NOT NULL,
    status friend_request_status NOT NULL DEFAULT 'Pending',
    apply_msg VARCHAR,
    req_remark VARCHAR,
    resp_msg VARCHAR,
    resp_remark VARCHAR,
    source VARCHAR,
    create_time BIGINT NOT NULL,
    update_time BIGINT,
    operator_id VARCHAR, -- 最后一次操作者ID
    last_operation VARCHAR, -- 最后一次操作类型
    deleted_time BIGINT, -- 删除时间，用于冷却期
    UNIQUE (user_id, friend_id)
);

-- 3. 创建好友分组表
CREATE TABLE friend_groups (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    create_time BIGINT NOT NULL,
    update_time BIGINT NOT NULL,
    sort_order INT DEFAULT 0,
    icon VARCHAR, -- 分组图标
    CONSTRAINT unique_user_group_name UNIQUE (user_id, name)
);

-- 4. 创建好友表
CREATE TABLE friends (
    id BIGSERIAL PRIMARY KEY,
    fs_id VARCHAR NOT NULL,
    user_id VARCHAR NOT NULL,
    friend_id VARCHAR NOT NULL,
    status friend_request_status NOT NULL DEFAULT 'Accepted',
    remark VARCHAR,
    source VARCHAR,
    create_time BIGINT NOT NULL,
    update_time BIGINT NOT NULL,
    deleted_time BIGINT, -- 删除时间，用于冷却期
    is_starred BOOLEAN DEFAULT FALSE, -- 是否星标好友
    group_id BIGINT, -- 所属分组
    priority INT DEFAULT 0, -- 联系人优先级
    CONSTRAINT unique_user_friend UNIQUE (user_id, friend_id)
);

-- 5. 创建好友标签表
CREATE TABLE friend_tags (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    color VARCHAR DEFAULT '#000000',
    create_time BIGINT NOT NULL,
    icon VARCHAR, -- 标签图标
    sort_order INT DEFAULT 0, -- 排序顺序
    CONSTRAINT unique_user_tag UNIQUE (user_id, name)
);

-- 6. 创建好友-标签关联表
CREATE TABLE friend_tag_relations (
    user_id VARCHAR NOT NULL,
    friend_id VARCHAR NOT NULL,
    tag_id BIGINT NOT NULL,
    create_time BIGINT NOT NULL, -- 添加创建时间
    PRIMARY KEY (user_id, friend_id, tag_id),
    FOREIGN KEY (tag_id) REFERENCES friend_tags (id) ON DELETE CASCADE
);

-- 7. 创建好友互动统计表（增加互动评分字段）
CREATE TABLE friend_interactions (
    user_id VARCHAR NOT NULL,
    friend_id VARCHAR NOT NULL,
    message_count INT DEFAULT 0,
    last_interact_time BIGINT,
    total_duration INT DEFAULT 0, -- 通话时长统计(秒)
    call_count INT DEFAULT 0, -- 通话次数
    interaction_score FLOAT DEFAULT 0.0, -- 互动亲密度评分
    last_week_count INT DEFAULT 0, -- 最近一周互动次数
    last_month_count INT DEFAULT 0, -- 最近一月互动次数
    PRIMARY KEY (user_id, friend_id),
    CONSTRAINT valid_interaction_counts CHECK (
        message_count >= 0
        AND call_count >= 0
        AND total_duration >= 0
    )
);

-- 8. 好友隐私设置表（优化隐私设置）
CREATE TABLE friend_privacy_settings (
    user_id VARCHAR NOT NULL,
    friend_id VARCHAR NOT NULL,
    can_see_moments BOOLEAN DEFAULT TRUE,
    can_see_online_status BOOLEAN DEFAULT TRUE,
    can_see_location BOOLEAN DEFAULT TRUE,
    can_see_mutual_friends BOOLEAN DEFAULT TRUE,
    permission_level VARCHAR DEFAULT 'full_access', -- 预定义权限级别
    custom_settings JSONB DEFAULT '{}', -- 自定义设置JSON
    PRIMARY KEY (user_id, friend_id)
);

-- 9. 好友备忘录表
CREATE TABLE friend_notes (
    user_id VARCHAR NOT NULL,
    friend_id VARCHAR NOT NULL,
    content TEXT NOT NULL,
    create_time BIGINT NOT NULL,
    update_time BIGINT NOT NULL,
    is_pinned BOOLEAN DEFAULT FALSE, -- 是否置顶
    PRIMARY KEY (user_id, friend_id)
);

-- 10. 添加基本索引
CREATE INDEX idx_friendships_fs_id ON friendships (id);

CREATE INDEX idx_friendships_user ON friendships (user_id);

CREATE INDEX idx_friendships_friend ON friendships (friend_id);

CREATE INDEX idx_friendships_status ON friendships (status);

CREATE INDEX idx_friends_fs_id ON friends (fs_id);

CREATE INDEX idx_friends_user ON friends (user_id);

CREATE INDEX idx_friends_friend ON friends (friend_id);

CREATE INDEX idx_friends_user_group ON friends (user_id, group_id);

CREATE INDEX idx_friend_groups_user ON friend_groups (user_id);

CREATE INDEX idx_friend_tags_user ON friend_tags (user_id);

CREATE INDEX idx_friend_interactions_user ON friend_interactions (user_id);

CREATE INDEX idx_friend_interactions_last_time ON friend_interactions (last_interact_time);

-- 11. 添加复合索引和部分索引
CREATE INDEX idx_friendships_user_status ON friendships (user_id, status);

CREATE INDEX idx_friends_user_starred ON friends (user_id, is_starred);

CREATE INDEX idx_friend_interactions_score ON friend_interactions (
    user_id,
    interaction_score DESC
);
-- 只对活跃状态的好友创建索引
CREATE INDEX idx_friends_active ON friends (user_id)
WHERE
    status = 'Accepted'
    AND deleted_time IS NULL;
-- 星标好友索引
CREATE INDEX idx_friends_starred ON friends (user_id)
WHERE
    is_starred = TRUE;

-- 12. 外键约束
ALTER TABLE friends
ADD CONSTRAINT fk_friend_group FOREIGN KEY (group_id) REFERENCES friend_groups (id);

ALTER TABLE friend_tag_relations
ADD CONSTRAINT fk_friend_relation_user_friend FOREIGN KEY (user_id, friend_id) REFERENCES friends (user_id, friend_id) ON DELETE CASCADE;

-- 13. 触发器函数：更新好友互动分数
CREATE OR REPLACE FUNCTION update_interaction_score()
RETURNS TRIGGER AS $$
BEGIN
    -- 计算互动评分：权重分配给最近互动和总互动量
    NEW.interaction_score := (0.5 * NEW.last_week_count) +
                            (0.3 * NEW.last_month_count) +
                            (0.2 * (NEW.message_count + NEW.call_count) / 100.0);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 14. 创建触发器
CREATE TRIGGER trg_update_interaction_score
BEFORE INSERT OR UPDATE ON friend_interactions
FOR EACH ROW
EXECUTE FUNCTION update_interaction_score();
