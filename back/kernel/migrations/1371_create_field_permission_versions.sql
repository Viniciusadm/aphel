CREATE TABLE field_permission_versions (
  record_type VARCHAR(64) NOT NULL,
  updated_at DATETIME(6) NOT NULL,
  updated_by CHAR(36) NULL,
  PRIMARY KEY (record_type)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
