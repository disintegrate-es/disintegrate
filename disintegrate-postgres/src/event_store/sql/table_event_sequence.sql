CREATE TABLE IF NOT EXISTS event_sequence (
    event_id bigint primary key generated always as identity,
    event_type varchar(255),
    consumed smallint DEFAULT 0 check (consumed <= 1),
    committed boolean DEFAULT false,
    inserted_at TIMESTAMP DEFAULT now()
);
