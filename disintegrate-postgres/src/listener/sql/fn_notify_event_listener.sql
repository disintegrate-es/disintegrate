CREATE OR REPLACE FUNCTION notify_event_listener() 
      RETURNS TRIGGER AS $$
 BEGIN
    PERFORM pg_notify(NEW.id, NEW.last_event_id::text);
    RETURN new;
 END;
$$ LANGUAGE plpgsql;
