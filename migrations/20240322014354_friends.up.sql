CREATE TABLE friends
(
    id            VARCHAR primary key,
    friendship_id VARCHAR               NOT NULL,
    user_id       VARCHAR               NOT NULL,
    friend_id     VARCHAR               NOT NULL,
    status        friend_request_status NOT NULL DEFAULT 'Pending',
    remark        VARCHAR,
    hello         VARCHAR,
    source        VARCHAR,
    create_time   timestamp             NOT NULL DEFAULT now(),
    update_time   timestamp             NOT NULL DEFAULT now(),
--     FOREIGN KEY (user_id) REFERENCES users (id),
--     FOREIGN KEY (friend_id) REFERENCES users (id),
    CONSTRAINT unique_user_friend UNIQUE (user_id, friend_id)
)-- Your SQL goes here
