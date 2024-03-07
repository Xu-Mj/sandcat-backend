CREATE TABLE group_members
(
    id           serial primary key,
    group_id     varchar   not null,
    user_id      varchar   not null,
    group_name   varchar(128),
    group_remark varchar(128),
    delivered    bool      not null default false,
    joined_at    timestamp not null default now(),
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (group_id) REFERENCES groups (id)
);
CREATE INDEX idx_group_members_group_id ON group_members (group_id);
-- when user login, we need to check if there is any unread 'create group' message
CREATE INDEX idx_group_members_user_id ON group_members (user_id, delivered);
