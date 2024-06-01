-- Add up migration script here
CREATE TABLE sequence (
    id      SERIAL PRIMARY KEY,
    max_seq BIGINT NOT NULL DEFAULT 0
);
