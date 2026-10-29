CREATE TABLE record_shares (
  id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  grantee_type VARCHAR(16) NOT NULL,
  grantee_id CHAR(36) NOT NULL,
  access VARCHAR(16) NOT NULL,
  shared_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY record_shares_grantee_unique (record_type, record_id, grantee_type, grantee_id),
  KEY record_shares_access_index (record_type, grantee_type, grantee_id, access, record_id),
  KEY record_shares_shared_by_fk_index (shared_by),
  CONSTRAINT record_shares_shared_by_fk FOREIGN KEY (shared_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
