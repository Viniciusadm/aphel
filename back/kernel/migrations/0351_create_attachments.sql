CREATE TABLE attachments (
  id CHAR(36) NOT NULL,
  file_id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY attachments_file_record_unique (file_id, record_type, record_id),
  KEY attachments_record_index (record_type, record_id, created_at),
  KEY attachments_created_by_index (created_by),
  CONSTRAINT attachments_file_fk FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE CASCADE,
  CONSTRAINT attachments_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
