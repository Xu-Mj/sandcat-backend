-- do not use foreign key, it has performance issue
CREATE TABLE messages
(
    send_id      VARCHAR   NOT NULL,
    receiver_id  VARCHAR   NOT NULL,
    local_id     VARCHAR   NOT NULL,
    server_id    VARCHAR   NOT NULL,
--     todo need a timestamp type
    send_time    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
--     seq          BIGINT,
    group_id     VARCHAR,
    msg_type     INT,
    content_type INT,
    content      BYTEA,
    is_read      BOOLEAN            DEFAULT FALSE,
    PRIMARY KEY (send_id, server_id, send_time)
) PARTITION BY RANGE (send_time);;
