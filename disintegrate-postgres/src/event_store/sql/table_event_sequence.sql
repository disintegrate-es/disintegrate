CREATE TABLE IF NOT EXISTS event_sequence (
    id bigint generated always as identity,
    event_type varchar(255),
    domain_identifiers jsonb,
    consumed smallint DEFAULT 0 check (consumed <= 1),
    inserted_at TIMESTAMP DEFAULT now()
);
