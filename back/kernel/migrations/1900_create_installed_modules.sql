CREATE TABLE installed_modules (
  module_key VARCHAR(32) NOT NULL,
  enabled BOOLEAN NOT NULL DEFAULT FALSE,
  settings JSON NOT NULL,
  installed_version VARCHAR(64) NOT NULL,
  enabled_at DATETIME(6) NULL,
  disabled_at DATETIME(6) NULL,
  data_exported_at DATETIME(6) NULL,
  data_export_file_id CHAR(36) NULL,
  data_purged_at DATETIME(6) NULL,
  updated_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (module_key),
  KEY installed_modules_updated_by_fk_index (updated_by),
  CONSTRAINT installed_modules_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
