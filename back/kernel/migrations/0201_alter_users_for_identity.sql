ALTER TABLE users
  MODIFY password_hash VARCHAR(255) NULL,
  ADD COLUMN email_verified_at DATETIME(6) NULL AFTER display_name,
  ADD COLUMN status VARCHAR(20) NOT NULL DEFAULT 'active' AFTER email_verified_at,
  ADD COLUMN locale VARCHAR(10) NULL AFTER status,
  ADD COLUMN timezone VARCHAR(64) NULL AFTER locale,
  ADD COLUMN avatar_file_id CHAR(36) NULL AFTER timezone;

UPDATE users SET status = IF(is_active, 'active', 'deactivated');

ALTER TABLE users DROP COLUMN is_active;
