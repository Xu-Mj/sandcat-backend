-- 群组角色类型
CREATE TYPE group_role AS ENUM('Owner', 'Admin', 'Member');

-- 群组表
CREATE TABLE groups (
    id VARCHAR PRIMARY KEY,
    owner VARCHAR(256) NOT NULL,
    name VARCHAR(256) NOT NULL,
    avatar TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    announcement TEXT NOT NULL DEFAULT '',
    create_time BIGINT NOT NULL,
    update_time BIGINT NOT NULL,
    max_members INTEGER NOT NULL DEFAULT 500,
    is_public BOOLEAN NOT NULL DEFAULT true,
    join_approval_required BOOLEAN NOT NULL DEFAULT false,
    category VARCHAR(50),
    tags TEXT[],
    mute_all BOOLEAN NOT NULL DEFAULT false,
    only_admin_post BOOLEAN NOT NULL DEFAULT false,
    invite_permission group_role NOT NULL DEFAULT 'Admin',
    pinned_messages TEXT[]
);

CREATE INDEX idx_groups_owner ON groups(owner);
CREATE INDEX idx_groups_category ON groups(category);

-- 群组成员表
CREATE TABLE group_members (
    group_id VARCHAR NOT NULL,
    user_id VARCHAR NOT NULL,
    group_name VARCHAR(128),
    group_remark VARCHAR(128),
    role group_role NOT NULL DEFAULT 'Member',
    joined_at BIGINT NOT NULL,
    is_muted BOOLEAN NOT NULL DEFAULT false,
    notification_level VARCHAR(20) NOT NULL DEFAULT 'all',
    last_read_time BIGINT NOT NULL DEFAULT 0,
    display_order INTEGER NOT NULL DEFAULT 0,
    joined_by VARCHAR(256),
    PRIMARY KEY (group_id, user_id)
);

CREATE INDEX idx_group_members_group_id ON group_members(group_id);
CREATE INDEX idx_group_members_user_id ON group_members(user_id);

-- 群组分类表
CREATE TABLE group_categories (
    id VARCHAR PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    description TEXT,
    display_order INTEGER NOT NULL DEFAULT 0,
    create_time BIGINT NOT NULL,
    update_time BIGINT NOT NULL
);

CREATE INDEX idx_group_categories_name ON group_categories(name);

-- 群组文件表
CREATE TABLE group_files (
    id VARCHAR PRIMARY KEY,
    group_id VARCHAR NOT NULL,
    uploader_id VARCHAR NOT NULL,
    file_name VARCHAR(256) NOT NULL,
    file_url TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    file_type VARCHAR(50) NOT NULL,
    upload_time BIGINT NOT NULL,
    is_pinned BOOLEAN NOT NULL DEFAULT false,
    download_count INTEGER NOT NULL DEFAULT 0,
    thumbnail_url TEXT,
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
);

CREATE INDEX idx_group_files_group_id ON group_files(group_id);
CREATE INDEX idx_group_files_uploader_id ON group_files(uploader_id);
CREATE INDEX idx_group_files_upload_time ON group_files(upload_time);

-- 群组投票表
CREATE TABLE group_polls (
    id VARCHAR PRIMARY KEY,
    group_id VARCHAR NOT NULL,
    creator_id VARCHAR NOT NULL,
    title VARCHAR(256) NOT NULL,
    options JSONB NOT NULL,
    results JSONB,
    is_multiple BOOLEAN NOT NULL DEFAULT false,
    is_anonymous BOOLEAN NOT NULL DEFAULT false,
    created_at BIGINT NOT NULL,
    expires_at BIGINT,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
);

CREATE INDEX idx_group_polls_group_id ON group_polls(group_id);
CREATE INDEX idx_group_polls_created_at ON group_polls(created_at);

-- 群组禁言记录表
CREATE TABLE group_mute_records (
    id VARCHAR PRIMARY KEY,
    group_id VARCHAR NOT NULL,
    user_id VARCHAR NOT NULL,
    operator_id VARCHAR NOT NULL,
    mute_until BIGINT NOT NULL,
    reason TEXT,
    created_at BIGINT NOT NULL,
    FOREIGN KEY (group_id, user_id) REFERENCES group_members(group_id, user_id) ON DELETE CASCADE
);

CREATE INDEX idx_group_mute_records_group_user ON group_mute_records(group_id, user_id);

-- 群组邀请表
CREATE TABLE group_invitations (
    id VARCHAR PRIMARY KEY,
    group_id VARCHAR NOT NULL,
    inviter_id VARCHAR NOT NULL,
    invitee_id VARCHAR NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at BIGINT NOT NULL,
    expires_at BIGINT,
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
);

CREATE INDEX idx_group_invitations_group_id ON group_invitations(group_id);
CREATE INDEX idx_group_invitations_invitee_id ON group_invitations(invitee_id);

-- 群组公告历史表
CREATE TABLE group_announcements (
    id VARCHAR PRIMARY KEY,
    group_id VARCHAR NOT NULL,
    creator_id VARCHAR NOT NULL,
    content TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    is_pinned BOOLEAN NOT NULL DEFAULT false,
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
);

CREATE INDEX idx_group_announcements_group_id ON group_announcements(group_id);
