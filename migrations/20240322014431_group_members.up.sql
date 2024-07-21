CREATE TYPE group_role AS ENUM('Owner', 'Admin', 'Member');
CREATE TABLE group_members
(
--     id           BIGSERIAL PRIMARY KEY,
    group_id     VARCHAR NOT NULL,
    user_id      VARCHAR NOT NULL,
    group_name   VARCHAR(128),
    group_remark VARCHAR(128),
    role         group_role NOT NULL DEFAULT 'Member',
    joined_at    BIGINT  NOT NULL,
    PRIMARY KEY (group_id, user_id)
);
CREATE INDEX idx_group_members_group_id ON group_members (group_id);
-- CREATE INDEX idx_group_members_user_id_delivered ON group_members (user_id, delivered);
CREATE INDEX idx_group_members_user_id ON group_members (user_id);
-- CREATE UNIQUE INDEX idx_group_members_user_id_and_group_id ON group_members (user_id, group_id);
