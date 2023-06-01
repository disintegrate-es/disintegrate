CREATE TABLE IF NOT EXISTS event_sequence (
    event_id bigint generated always as identity,
    event_type varchar(255),
    consumed smallint DEFAULT 0 check (consumed <= 1),
    inserted_at TIMESTAMP DEFAULT now()
);
