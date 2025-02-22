CREATE OR REPLACE FUNCTION event_store_begin_epoch() 
RETURNS void AS $$
DECLARE
    id BIGINT;
    db_id INT;
BEGIN
    -- Fetch the maximum event id, default to 0 if no events exist
    SELECT COALESCE(MAX(event_id), 0) INTO id FROM event;
    SELECT oid INTO db_id FROM pg_database WHERE datname = current_database();

    PERFORM pg_try_advisory_xact_lock_shared(db_id, 0);
    PERFORM pg_try_advisory_xact_lock_shared(1, (id & 0xFFFFFFFF)::bit(32)::integer);
    PERFORM pg_try_advisory_xact_lock_shared(2, (id >> 32)::bit(32)::integer);
END;
$$ LANGUAGE plpgsql;