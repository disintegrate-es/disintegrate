CREATE INDEX IF NOT EXISTS idx_event_sequence_type ON event_sequence USING HASH (event_type);
