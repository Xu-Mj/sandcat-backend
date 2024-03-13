CREATE TABLE group_members
(
    id           BIGSERIAL PRIMARY KEY,
    group_id     VARCHAR   NOT NULL,
    user_id      VARCHAR   NOT NULL,
    group_name   VARCHAR(128),
    group_remark VARCHAR(128),
    delivered    bool      NOT NULL DEFAULT FALSE,
    joined_at    timestamp NOT NULL DEFAULT now(),
    FOREIGN KEY (user_id) REFERENCES users (id),
    -- when group deleted, all members which belong to that group will be deleted
    FOREIGN KEY (group_id) REFERENCES groups (id) ON DELETE CASCADE
);
CREATE INDEX idx_group_members_group_id ON group_members (group_id);
-- when user login, we need to check if there is any unread 'create group' message
CREATE INDEX idx_group_members_user_id_delivered ON group_members (user_id, delivered);
CREATE INDEX idx_group_members_user_id ON group_members (user_id);
CREATE UNIQUE INDEX idx_group_members_user_id_and_group_id ON group_members (user_id, group_id);
