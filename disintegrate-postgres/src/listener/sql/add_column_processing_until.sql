ALTER TABLE event_listener
ADD COLUMN IF NOT EXISTS processing_until TIMESTAMP;
