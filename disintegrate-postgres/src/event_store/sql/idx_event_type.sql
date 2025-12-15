CREATE INDEX IF NOT EXISTS idx_event_type_btree ON event USING btree (event_type);
