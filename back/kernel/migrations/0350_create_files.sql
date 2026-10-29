CREATE TABLE files (
  id CHAR(36) NOT NULL,
  storage_key VARCHAR(255) NOT NULL,
  original_name VARCHAR(255) NOT NULL,
  mime_type VARCHAR(127) NOT NULL,
  size BIGINT UNSIGNED NOT NULL,
  checksum CHAR(64) NOT NULL,
  uploaded_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  deleted_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY files_storage_key_unique (storage_key),
  KEY files_created_at_index (created_at),
  KEY files_uploaded_by_index (uploaded_by),
  CONSTRAINT files_uploaded_by_fk FOREIGN KEY (uploaded_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
