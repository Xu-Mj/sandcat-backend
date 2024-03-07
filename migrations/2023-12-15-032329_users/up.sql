CREATE TABLE users
(
    id          VARCHAR PRIMARY KEY,
    name        VARCHAR   NOT NULL,
    account     VARCHAR   NOT NULL,
    password    VARCHAR   NOT NULL,
    avatar      VARCHAR   NOT NULL,
    gender      VARCHAR   NOT NULL,
    age         int       NOT NULL DEFAULT 0,
    phone       VARCHAR(20),
    email       VARCHAR(64),
    address     VARCHAR(1024),
    birthday    timestamp          DEFAULT now(),
    create_time timestamp NOT NULL DEFAULT now(),
    update_time timestamp NOT NULL DEFAULT now(),
    is_delete   boolean   NOT NULL DEFAULT FALSE
);
