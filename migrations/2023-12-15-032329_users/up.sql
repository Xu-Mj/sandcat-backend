-- Your SQL goes here
CREATE TABLE users
(
    id          varchar PRIMARY KEY,
    name        varchar   not null,
    account     varchar   not null,
    password    varchar   not null,
    avatar      varchar   not null,
    gender      varchar   not null,
    age         int       not null default 0,
    phone       varchar(20),
    email       varchar(64),
    address     varchar(1024),
    birthday    timestamp          default now(),
    create_time timestamp not null default now(),
    update_time timestamp not null default now(),
    is_delete   boolean   not null default false
);
