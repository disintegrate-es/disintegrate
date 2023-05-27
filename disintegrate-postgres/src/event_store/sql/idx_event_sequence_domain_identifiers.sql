CREATE INDEX IF NOT EXISTS idx_event_sequence_domain_identifiers ON event_sequence USING gin(domain_identifiers);
