ALTER TABLE site_previews
  ADD COLUMN record_type VARCHAR(100) NULL AFTER theme,
  ADD COLUMN record_id CHAR(36) NULL AFTER record_type,
  ADD COLUMN draft_fields TEXT NULL AFTER record_id,
  ADD COLUMN target_path VARCHAR(500) NULL AFTER draft_fields;
