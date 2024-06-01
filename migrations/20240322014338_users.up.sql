CREATE TABLE users
(
    id          BIGSERIAL PRIMARY KEY AUTO_INCREMENT,
    name        VARCHAR NOT NULL,
    account     VARCHAR NOT NULL,
    password    VARCHAR NOT NULL,
    salt        VARCHAR NOT NULL,
    signature   VARCHAR(1024),
    avatar      VARCHAR NOT NULL,
    gender      VARCHAR NOT NULL,
    age         INT     NOT NULL DEFAULT 0,
    phone       VARCHAR(20),
    email       VARCHAR(64),
    address     VARCHAR(1024),
    region      VARCHAR(1024),
    birthday    BIGINT,
    create_time BIGINT  NOT NULL,
    update_time BIGINT  NOT NULL,
    is_delete   boolean NOT NULL DEFAULT FALSE
);
