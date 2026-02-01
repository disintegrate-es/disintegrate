CREATE INDEX IF NOT EXISTS idx_domain_ids ON event USING GIN (domain_ids);
