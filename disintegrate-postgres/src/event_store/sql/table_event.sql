CREATE TABLE IF NOT EXISTS event (
    event_id BIGINT PRIMARY KEY DEFAULT nextval('seq_event_event_id'),
    event_type VARCHAR(255),
    payload BYTEA,
    inserted_at TIMESTAMP DEFAULT now()
);
