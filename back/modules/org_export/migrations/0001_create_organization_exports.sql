CREATE TABLE organization_exports (
  id CHAR(36) NOT NULL,
  status VARCHAR(16) NOT NULL,
  include_files TINYINT(1) NOT NULL DEFAULT 1,
  file_id CHAR(36) NULL,
  summary JSON NULL,
  failure_reason VARCHAR(500) NULL,
  requested_by CHAR(36) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  started_at DATETIME(6) NULL,
  finished_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  KEY organization_exports_created_at_index (created_at),
  KEY organization_exports_file_fk_index (file_id),
  KEY organization_exports_user_fk_index (requested_by),
  CONSTRAINT organization_exports_file_fk FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE SET NULL,
  CONSTRAINT organization_exports_requested_by_fk FOREIGN KEY (requested_by) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
