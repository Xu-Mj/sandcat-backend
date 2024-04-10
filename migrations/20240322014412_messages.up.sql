-- do not use foreign key, it has performance issue
CREATE TABLE messages
(
    send_id      VARCHAR NOT NULL,
    receiver_id  VARCHAR NOT NULL,
    local_id     VARCHAR NOT NULL,
    server_id    VARCHAR NOT NULL,
--     todo need a timestamp type
--     create_time    BIGINT  NOT NULL,
    send_time    BIGINT  NOT NULL,
--     seq          BIGINT,
    msg_type     INT,
    content_type INT,
    content      BYTEA,
--     is_read      BOOLEAN DEFAULT FALSE,
    PRIMARY KEY (send_id, server_id, send_time)
) /*PARTITION BY RANGE (send_time)*/;
