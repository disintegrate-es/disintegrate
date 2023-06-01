CREATE TABLE IF NOT EXISTS event (
    event_id bigint PRIMARY KEY,
    event_type varchar(255),
    payload bytea,
    inserted_at TIMESTAMP DEFAULT now()
);
