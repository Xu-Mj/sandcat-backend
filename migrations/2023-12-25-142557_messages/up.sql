CREATE TABLE messages
(
    msg_id       VARCHAR PRIMARY KEY,
    msg_type     VARCHAR   NOT NULL,
    content      VARCHAR   NOT NULL,
    content_type VARCHAR   NOT NULL,
    send_id      VARCHAR   NOT NULL,
    friend_id    VARCHAR   NOT NULL,
    is_read      bool      NOT NULL DEFAULT FALSE,
    delivered    bool      NOT NULL DEFAULT FALSE,
    create_time  TIMESTAMP NOT NULL DEFAULT now(),
    FOREIGN KEY (send_id) REFERENCES users (id),
    FOREIGN KEY (friend_id) REFERENCES users (id)
)-- Your SQL goes here
