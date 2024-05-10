CREATE TABLE messages
(
    send_id      VARCHAR NOT NULL,
    receiver_id  VARCHAR NOT NULL,
    local_id     VARCHAR NOT NULL,
    server_id    VARCHAR NOT NULL,
    send_time    BIGINT  NOT NULL,
    msg_type     INT,
    content_type INT,
    content      BYTEA,
    PRIMARY KEY (send_id, server_id, send_time)
) PARTITION BY RANGE (send_time);

DO $$
    DECLARE
        week_start_ms BIGINT;
        week_end_ms BIGINT;
        current_week_start TIMESTAMP;
        year_week_num VARCHAR(10);
    BEGIN
        -- 获取当前时间戳所在周的开始时间（使用毫秒）
        current_week_start := DATE_TRUNC('week', NOW());
        FOR i IN 0..51 LOOP
                -- 计算每个分区的开始和结束时间戳（毫秒）
                week_start_ms := (EXTRACT(EPOCH FROM current_week_start + (i * INTERVAL '1 week')) * 1000)::BIGINT;
                week_end_ms := (EXTRACT(EPOCH FROM current_week_start + ((i + 1) * INTERVAL '1 week')) * 1000)::BIGINT;

                -- 年份和周数标识
                year_week_num := TO_CHAR(current_week_start + (i * INTERVAL '1 week'), 'IYYY_IW');

                -- 创建分区表
                EXECUTE 'CREATE TABLE IF NOT EXISTS messages_' || year_week_num || ' PARTITION OF messages FOR VALUES FROM (' || week_start_ms || ') TO (' || week_end_ms || ')';
            END LOOP;
    END
$$;
