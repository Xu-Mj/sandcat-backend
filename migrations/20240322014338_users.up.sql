CREATE TABLE users
(
    id                  TEXT PRIMARY KEY,
    name                TEXT NOT NULL,
    account             TEXT NOT NULL,
    password            TEXT NOT NULL,
    salt                TEXT NOT NULL,
    signature           TEXT,
    avatar              TEXT NOT NULL,
    gender              TEXT NOT NULL,
    age                 INTEGER NOT NULL DEFAULT 0,
    phone               TEXT,
    email               TEXT,
    address             TEXT,
    region              TEXT,
    birthday            BIGINT,
    create_time         BIGINT NOT NULL,
    update_time         BIGINT NOT NULL,

    -- 账号与安全
    last_login_time     BIGINT,
    last_login_ip       TEXT,
    two_factor_enabled  BOOLEAN NOT NULL DEFAULT FALSE,
    account_status      TEXT NOT NULL DEFAULT 'active',

    -- 在线状态管理
    status              TEXT NOT NULL DEFAULT 'offline',
    last_active_time    BIGINT,
    status_message      TEXT,

    -- 隐私与设置
    privacy_settings    TEXT NOT NULL DEFAULT '{}',
    notification_settings TEXT NOT NULL DEFAULT '{}',
    language            TEXT NOT NULL DEFAULT 'zh_CN',

    -- 社交与关系
    friend_requests_privacy TEXT NOT NULL DEFAULT 'all',
    profile_visibility     TEXT NOT NULL DEFAULT 'public',

    -- 用户体验
    theme               TEXT NOT NULL DEFAULT 'system',
    timezone            TEXT NOT NULL DEFAULT 'Asia/Shanghai',

    is_delete           BOOLEAN NOT NULL DEFAULT FALSE
);

-- 索引保持不变
CREATE INDEX idx_users_account ON users(account) WHERE NOT is_delete;
CREATE INDEX idx_users_email ON users(email) WHERE NOT is_delete;
CREATE INDEX idx_users_phone ON users(phone) WHERE NOT is_delete;
CREATE INDEX idx_users_status ON users(status) WHERE NOT is_delete;
