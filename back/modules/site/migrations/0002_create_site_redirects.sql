CREATE TABLE site_redirects (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  record_type VARCHAR(64) NOT NULL,
  field VARCHAR(64) NOT NULL,
  old_value VARCHAR(255) NOT NULL,
  new_value VARCHAR(255) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY site_redirects_source_unique (record_type, field, old_value),
  KEY site_redirects_target_index (record_type, field, new_value)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
