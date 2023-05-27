CREATE INDEX IF NOT EXISTS idx_event_listener_event_types ON event_listener using gin(event_types);
