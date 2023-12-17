-- Your SQL goes here
CREATE TABLE users
(
    id          serial PRIMARY KEY,
    name        varchar   not null,
    account     varchar   not null,
    avatar      varchar   not null,
    gender      varchar   not null,
    phone       varchar(20),
    email       varchar(64),
    address     varchar(1024),
    birthday    timestamp          default now(),
    create_time timestamp not null default now(),
    update_time timestamp not null default now(),
    is_delete   boolean   not null default false
);
