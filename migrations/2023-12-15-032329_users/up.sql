-- Your SQL goes here
CREATE TABLE users
(
    id          serial PRIMARY KEY,
    name        varchar,
    account     varchar       not null,
    avatar      varchar       not null,
    gender      varchar       not null,
    phone       varchar(20)   not null,
    email       varchar(64)   not null,
    address     varchar(1024) not null,
    birthday    timestamp     not null default now(),
    create_time timestamp     not null default now(),
    update_time timestamp     not null default now(),
    is_delete   boolean       not null default false
);
