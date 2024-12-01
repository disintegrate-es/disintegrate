CREATE INDEX IF NOT EXISTS idx_events_type ON event USING HASH (event_type);
