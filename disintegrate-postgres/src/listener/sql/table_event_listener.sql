CREATE TABLE IF NOT EXISTS event_listener (
    id TEXT PRIMARY KEY,
    last_processed_event_id BIGINT,
    processing_until TIMESTAMP,
    updated_at TIMESTAMP DEFAULT now()
);

