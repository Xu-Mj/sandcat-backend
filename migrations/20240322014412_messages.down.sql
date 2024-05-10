-- This file should undo anything in `up.sql`
DO
$$
    DECLARE
        partition RECORD;
    BEGIN
        FOR partition IN SELECT tablename FROM pg_catalog.pg_tables WHERE tablename LIKE 'messages%'
            LOOP
                EXECUTE 'DROP TABLE IF EXISTS ' || quote_ident(partition.tablename) || ' CASCADE;';
            END LOOP;
    END
$$;
