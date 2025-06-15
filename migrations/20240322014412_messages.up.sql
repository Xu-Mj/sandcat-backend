-- Optimized messages table for audit/backup purposes
CREATE TABLE messages
(
    -- Basic identifiers (required)
    sender_id        VARCHAR NOT NULL,
    receiver_id    VARCHAR NOT NULL,
    client_id      VARCHAR NOT NULL,
    server_id      VARCHAR NOT NULL,
    send_time      BIGINT  NOT NULL,

    -- Core classification fields (for audit queries)
    msg_type       SMALLINT,
    content_type   SMALLINT,
    group_id       VARCHAR,
    platform       SMALLINT,

    -- Complete message content (binary)
    content        BYTEA NOT NULL,

    PRIMARY KEY (sender_id, server_id, send_time)
) PARTITION BY RANGE (send_time);

-- Indexes for common audit queries
CREATE INDEX idx_messages_receiver ON messages(receiver_id, send_time DESC);
CREATE INDEX idx_messages_group ON messages(group_id, send_time DESC) WHERE group_id IS NOT NULL;
CREATE INDEX idx_messages_time ON messages(send_time DESC);

-- Function to maintain partitions - creates future partitions and purges old ones
CREATE OR REPLACE FUNCTION maintain_message_partitions(
    future_weeks INT DEFAULT 4,
    retention_weeks INT DEFAULT 52
)
RETURNS void AS $$
DECLARE
    week_start_ms BIGINT;
    week_end_ms BIGINT;
    current_week_start TIMESTAMP;
    year_week_num VARCHAR(10);
    old_partition TEXT;
    partitions_created INT := 0;
    partitions_dropped INT := 0;
BEGIN
    -- Get current week start
    current_week_start := DATE_TRUNC('week', NOW());

    -- Create future partitions
    FOR i IN 0..future_weeks LOOP
        BEGIN
            -- Calculate partition time boundaries (milliseconds)
            week_start_ms := (EXTRACT(EPOCH FROM current_week_start + (i * INTERVAL '1 week')) * 1000)::BIGINT;
            week_end_ms := (EXTRACT(EPOCH FROM current_week_start + ((i + 1) * INTERVAL '1 week')) * 1000)::BIGINT;

            -- Year and week identifier
            year_week_num := TO_CHAR(current_week_start + (i * INTERVAL '1 week'), 'IYYY_IW');

            -- Create partition with optimized storage parameters
            EXECUTE 'CREATE TABLE IF NOT EXISTS messages_' || year_week_num ||
                    ' PARTITION OF messages FOR VALUES FROM (' || week_start_ms ||
                    ') TO (' || week_end_ms || ')' ||
                    ' WITH (fillfactor=90)';

            partitions_created := partitions_created + 1;
        EXCEPTION WHEN OTHERS THEN
            RAISE WARNING 'Failed to create partition for week %: %', year_week_num, SQLERRM;
        END;
    END LOOP;

    -- Drop old partitions beyond retention period
    FOR old_partition IN
        SELECT tablename FROM pg_tables
        WHERE tablename LIKE 'messages_%'
        AND tablename < 'messages_' || TO_CHAR(current_week_start - ((retention_weeks) * INTERVAL '1 week'), 'IYYY_IW')
    LOOP
        BEGIN
            EXECUTE 'DROP TABLE IF EXISTS ' || old_partition;
            partitions_dropped := partitions_dropped + 1;
        EXCEPTION WHEN OTHERS THEN
            RAISE WARNING 'Failed to drop old partition %: %', old_partition, SQLERRM;
        END;
    END LOOP;

    RAISE NOTICE 'Partition maintenance complete: % created, % dropped',
                  partitions_created, partitions_dropped;
END;
$$ LANGUAGE plpgsql;

-- Initial partition creation
DO $$
BEGIN
    -- Create initial year of partitions
    PERFORM maintain_message_partitions(52, 52);

    -- Log completion
    RAISE NOTICE 'Initial partition setup complete';
EXCEPTION WHEN OTHERS THEN
    RAISE WARNING 'Failed during initial partition setup: %', SQLERRM;
END
$$;

-- View for monitoring partition status
CREATE OR REPLACE VIEW message_partition_info AS
SELECT
    tablename,
    pg_size_pretty(pg_total_relation_size(tablename::regclass)) as size,
    pg_stat_get_numscans(tablename::regclass) as scan_count,
    to_timestamp(SPLIT_PART(REPLACE(tablename, 'messages_', ''), '_', 1)::integer * 7 * 24 * 60 * 60) as week_start_date
FROM pg_tables
WHERE tablename LIKE 'messages_%'
ORDER BY tablename;

-- Optional: Setup automatic maintenance using pg_cron
-- Uncomment if pg_cron extension is available
-- CREATE EXTENSION IF NOT EXISTS pg_cron;
-- SELECT cron.schedule('0 0 * * 0', 'SELECT maintain_message_partitions()');

-- Create partition cleanup function that can be called manually or by cron job
CREATE OR REPLACE FUNCTION cleanup_old_message_partitions(weeks_to_keep INT DEFAULT 52)
RETURNS INT AS $$
DECLARE
    dropped_count INT := 0;
BEGIN
    PERFORM maintain_message_partitions(4, weeks_to_keep);
    RETURN dropped_count;
END;
$$ LANGUAGE plpgsql;

-- Comment to document table purpose
COMMENT ON TABLE messages IS 'Message audit/backup storage with weekly partitioning';
