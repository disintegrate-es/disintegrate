CREATE OR REPLACE FUNCTION event_store_current_epoch()
RETURNS BIGINT AS $$
DECLARE
    persisted_event_id BIGINT;
    pending_event_id BIGINT;
    db_id INT;
BEGIN
    SELECT COALESCE(MAX(event_id), 0) INTO persisted_event_id FROM event;
    SELECT oid INTO db_id FROM pg_database WHERE datname = current_database();

    SELECT MIN((l3.objid::bigint << 32) + l2.objid::bigint)
    INTO pending_event_id
    FROM pg_locks l1
    INNER JOIN pg_locks l2 ON l1.pid = l2.pid
    INNER JOIN pg_locks l3 ON l1.pid = l3.pid
    WHERE 
        l1.classid = db_id
        AND l2.classid = 1
        AND l3.classid = 2
        AND l1.locktype = 'advisory';
    
    RETURN COALESCE(pending_event_id, persisted_event_id);
END;
$$ LANGUAGE plpgsql;
