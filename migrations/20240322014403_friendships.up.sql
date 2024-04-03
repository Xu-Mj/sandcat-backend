CREATE TYPE friend_request_status AS ENUM ('Pending', 'Accepted', 'Rejected', 'Blacked', 'Cancelled', 'Deleted');
create table friendships
(
    id           VARCHAR primary key,
    user_id      VARCHAR               NOT NULL,
    friend_id    VARCHAR               NOT NULL,
    status       friend_request_status NOT NULL DEFAULT 'Pending',
    apply_msg    VARCHAR,
    req_remark   VARCHAR,
    response_msg VARCHAR,
    res_remark   VARCHAR,
    source       VARCHAR,
    create_time  timestamp             NOT NULL DEFAULT now(),
    accept_time  timestamp             NOT NULL DEFAULT now(),
    Unique (user_id, friend_id)
)
