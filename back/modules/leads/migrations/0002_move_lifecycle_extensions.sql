CREATE TABLE IF NOT EXISTS leads_contact_extension (
  record_id CHAR(36) NOT NULL PRIMARY KEY,
  lifecycle_stage VARCHAR(64) NULL,
  KEY leads_contact_stage_index (lifecycle_stage)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS leads_company_extension (
  record_id CHAR(36) NOT NULL PRIMARY KEY,
  lifecycle_stage VARCHAR(64) NULL,
  KEY leads_company_stage_index (lifecycle_stage)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

SET @copy_extension = IF((SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'contacts' AND COLUMN_NAME = 'lifecycle_stage') > 0, 'INSERT IGNORE INTO leads_contact_extension (record_id, lifecycle_stage) SELECT id, lifecycle_stage FROM contacts WHERE lifecycle_stage IS NOT NULL', 'SELECT 1');
PREPARE copy_extension FROM @copy_extension;
EXECUTE copy_extension;
DEALLOCATE PREPARE copy_extension;
SET @copy_extension = IF((SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'companies' AND COLUMN_NAME = 'lifecycle_stage') > 0, 'INSERT IGNORE INTO leads_company_extension (record_id, lifecycle_stage) SELECT id, lifecycle_stage FROM companies WHERE lifecycle_stage IS NOT NULL', 'SELECT 1');
PREPARE copy_extension FROM @copy_extension;
EXECUTE copy_extension;
DEALLOCATE PREPARE copy_extension;
