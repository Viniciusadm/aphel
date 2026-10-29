ALTER TABLE sessions
  ADD COLUMN stage VARCHAR(32) NOT NULL DEFAULT 'full' AFTER token_hash;
