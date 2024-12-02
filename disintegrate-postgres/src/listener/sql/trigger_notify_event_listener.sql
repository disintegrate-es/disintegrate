CREATE OR REPLACE TRIGGER event_insert_trigger
  AFTER INSERT ON event 
  FOR EACH ROW
  EXECUTE function notify_event_listener();
