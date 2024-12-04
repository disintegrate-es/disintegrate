CREATE OR REPLACE FUNCTION notify_event_listener()
      RETURNS TRIGGER AS $$
 BEGIN
    PERFORM pg_notify('new_events', NEW.event_type);
    RETURN new;
 END;
$$ LANGUAGE plpgsql;

