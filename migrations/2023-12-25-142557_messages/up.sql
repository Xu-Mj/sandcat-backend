create table messages
(
    msg_id       varchar primary key,
    msg_type     varchar   not null,
    content      varchar   not null,
    content_type varchar   not null,
    send_id      varchar   not null,
    friend_id    varchar   not null,
    is_read      bool      not null default false,
    delivered    bool      not null default false,
    create_time  timestamp not null default now(),
    FOREIGN KEY (send_id) REFERENCES users (id),
    FOREIGN KEY (friend_id) REFERENCES users (id)
)-- Your SQL goes here
