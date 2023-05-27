CREATE TABLE IF NOT EXISTS event (
    id bigint PRIMARY KEY,
    event_type varchar(255),
    domain_identifiers jsonb,
    payload bytea,
    inserted_at TIMESTAMP DEFAULT now()
);
