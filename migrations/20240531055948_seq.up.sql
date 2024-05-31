-- Add up migration script here
CREATE TABLE sequence (
    user_id VARCHAR PRIMARY KEY,
    max_seq BIGINT NOT NULL DEFAULT 0
);
