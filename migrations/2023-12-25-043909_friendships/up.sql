create table friendships
(
    id          varchar primary key,
    user_id     varchar   not null,
    friend_id   varchar   not null,
    status      char      not null default '0',
    apply_msg   varchar,
    source        varchar,
    create_time timestamp not null default now(),
    update_time timestamp not null default now(),
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (friend_id) REFERENCES users (id)
)-- Your SQL goes here
-- Your SQL goes here
