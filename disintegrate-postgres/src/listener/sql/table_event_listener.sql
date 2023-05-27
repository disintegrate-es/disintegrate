CREATE TABLE IF NOT EXISTS event_listener (
    id TEXT PRIMARY KEY,
    event_types TEXT[],
    last_event_id BIGSERIAL
);
