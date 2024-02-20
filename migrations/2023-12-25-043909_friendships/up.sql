create table friendships
(
    id           varchar primary key,
    user_id      varchar   not null,
    friend_id    varchar   not null,
    status       char      not null default '0',
    apply_msg    varchar,
    req_remark   varchar,
    response_msg varchar,
    res_remark   varchar,
    source       varchar,
    is_delivered bool      not null default false,
    create_time  timestamp not null default now(),
    update_time  timestamp not null default now(),
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (friend_id) REFERENCES users (id),
    Unique (user_id, friend_id)
)
-- Your SQL goes here
-- Your SQL goes here
