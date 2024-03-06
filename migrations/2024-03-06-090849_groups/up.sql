-- groups table creation sql
CREATE TABLE groups
(
    id           VARCHAR PRIMARY KEY,
    owner        VARCHAR      NOT NULL,
    name         VARCHAR(255) NOT NULL,
    members      TEXT         NOT NULL,
    avatar       TEXT         NOT NULL,
    description  TEXT         NOT NULL DEFAULT '',
    announcement TEXT         NOT NULL DEFAULT '',
    create_time  TIMESTAMP    NOT NULL DEFAULT NOW(),
    FOREIGN KEY (owner) REFERENCES users (id)
);
