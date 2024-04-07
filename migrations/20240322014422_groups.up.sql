-- groups table creation sql
-- do not use foreign key constraint
CREATE TABLE groups
(
    id           VARCHAR PRIMARY KEY,
    owner        VARCHAR(256) NOT NULL,
    name         VARCHAR(256) NOT NULL,
    avatar       TEXT         NOT NULL,
    description  TEXT         NOT NULL DEFAULT '',
    announcement TEXT         NOT NULL DEFAULT '',
    create_time  BIGINT    NOT NULL,
    update_time  BIGINT    NOT NULL
--     FOREIGN KEY (owner) REFERENCES users (id)
);
