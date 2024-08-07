CREATE TYPE friend_request_status AS ENUM(
    'Pending',
    'Accepted',
    'Rejected',
    'Blacked',
    'Deleted'
);

CREATE TABLE friends (
    id BIGSERIAL primary key,
    fs_id VARCHAR NOT NULL,
    user_id VARCHAR NOT NULL,
    friend_id VARCHAR NOT NULL,
    status friend_request_status NOT NULL DEFAULT 'Accepted',
    remark VARCHAR,
    source VARCHAR,
    create_time BIGINT NOT NULL,
    update_time BIGINT NOT NULL,
    CONSTRAINT unique_user_friend UNIQUE (user_id, friend_id)
);

CREATE INDEX idx_friends_fs_id ON friends (fs_id);
