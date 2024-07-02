CREATE TABLE sequence (
    user_id VARCHAR PRIMARY KEY,
    send_max_seq BIGINT  NOT NULL DEFAULT 0,
    rec_max_seq BIGINT  NOT NULL DEFAULT 0
);
