CREATE TABLE system_upgrades (
  id CHAR(36) NOT NULL,
  from_version VARCHAR(64) NULL,
  to_version VARCHAR(64) NOT NULL,
  from_migration BIGINT NULL,
  to_migration BIGINT NULL,
  status VARCHAR(16) NOT NULL,
  applied_migrations JSON NOT NULL,
  error TEXT NULL,
  hostname VARCHAR(255) NULL,
  backup_confirmed TINYINT(1) NOT NULL,
  started_at DATETIME(6) NOT NULL,
  finished_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  KEY system_upgrades_started_index (started_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
