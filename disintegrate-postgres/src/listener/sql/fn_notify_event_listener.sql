CREATE OR REPLACE FUNCTION notify_event_listener()
      RETURNS TRIGGER AS $$
 BEGIN
    PERFORM pg_notify('new_events', '1');
    RETURN new;
 END;
$$ LANGUAGE plpgsql;

