CREATE OR REPLACE TRIGGER trigger_notify_event_listener
  AFTER UPDATE ON event_listener 
  FOR EACH ROW
  EXECUTE PROCEDURE notify_event_listener();
