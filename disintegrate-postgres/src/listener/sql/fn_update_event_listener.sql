CREATE OR REPLACE FUNCTION update_event_listener()
RETURNS TRIGGER as $$
BEGIN
    UPDATE event_listener 
    SET last_event_id = new.id
    WHERE new.event_type = any(event_types);
    RETURN new;
END;
$$ language plpgsql;
