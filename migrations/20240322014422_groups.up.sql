-- groups table creation sql
CREATE TABLE groups
(
    id           VARCHAR PRIMARY KEY,
    owner        VARCHAR(256)      NOT NULL,
    name         VARCHAR(256) NOT NULL,
    avatar       TEXT         NOT NULL,
    description  TEXT         NOT NULL DEFAULT '',
    announcement TEXT         NOT NULL DEFAULT '',
    create_time  TIMESTAMP    NOT NULL DEFAULT NOW(),
    update_time  TIMESTAMP    NOT NULL DEFAULT NOW(),
    FOREIGN KEY (owner) REFERENCES users (id)
);
