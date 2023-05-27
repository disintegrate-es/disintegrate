CREATE OR REPLACE TRIGGER event_insert_trigger
AFTER INSERT ON event 
FOR EACH ROW
EXECUTE function update_event_listener();
