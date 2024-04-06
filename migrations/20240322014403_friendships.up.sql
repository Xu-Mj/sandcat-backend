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
    create_time  BIGINT                NOT NULL,
    accept_time  BIGINT,
    Unique (user_id, friend_id)
)
