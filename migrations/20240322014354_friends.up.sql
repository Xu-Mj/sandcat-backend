CREATE TABLE friends (
    id BIGINT primary key,
    -- friendship_id VARCHAR NOT NULL,
    user_id VARCHAR NOT NULL,
    friend_id VARCHAR NOT NULL,
    status friend_request_status NOT NULL DEFAULT 'Accepted',
    remark VARCHAR,
    source VARCHAR,
    create_time timestamp NOT NULL DEFAULT now(),
    update_time timestamp NOT NULL DEFAULT now(),
    CONSTRAINT unique_user_friend UNIQUE (user_id, friend_id)
)
