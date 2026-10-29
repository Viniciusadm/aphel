ALTER TABLE domain_events
  ADD COLUMN origin_type VARCHAR(64) NULL AFTER causation,
  ADD COLUMN origin_id VARCHAR(64) NULL AFTER origin_type,
  ADD KEY domain_events_origin_index (origin_type, origin_id);
