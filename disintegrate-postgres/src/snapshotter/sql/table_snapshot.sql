CREATE TABLE IF NOT EXISTS snapshot (
    id uuid PRIMARY KEY,
    name text,
    query text,
    version bigint,
    payload text,
    inserted_at TIMESTAMP DEFAULT now()
);
