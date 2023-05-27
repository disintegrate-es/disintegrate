CREATE INDEX IF NOT EXISTS idx_events_domain_identifiers ON event USING gin(domain_identifiers);
