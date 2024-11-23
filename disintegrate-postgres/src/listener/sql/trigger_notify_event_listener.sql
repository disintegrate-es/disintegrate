CREATE OR REPLACE TRIGGER trigger_notify_event_listener
  AFTER INSERT ON event
  FOR EACH STATEMENT
  EXECUTE PROCEDURE notify_event_listener();
