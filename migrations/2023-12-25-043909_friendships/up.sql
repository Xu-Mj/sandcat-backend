create table friendships
(
    id           VARCHAR primary key,
    user_id      VARCHAR   NOT NULL ,
    friend_id    VARCHAR   NOT NULL ,
    status       friend_request_status NOT NULL DEFAULT 'Pending',
    apply_msg    VARCHAR,
    req_remark   VARCHAR,
    response_msg VARCHAR,
    res_remark   VARCHAR,
    source       VARCHAR,
    is_delivered bool      NOT NULL DEFAULT false,
    create_time  timestamp NOT NULL DEFAULT now(),
    update_time  timestamp NOT NULL DEFAULT now(),
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (friend_id) REFERENCES users (id),
    Unique (user_id, friend_id)
)
-- Your SQL goes here
-- Your SQL goes here
