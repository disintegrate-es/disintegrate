CREATE TABLE IF NOT EXISTS event_listener (
    id TEXT PRIMARY KEY,
    last_processed_event_id BIGINT,
    updated_at TIMESTAMP DEFAULT now()
);

