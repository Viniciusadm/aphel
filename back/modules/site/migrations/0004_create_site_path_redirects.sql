CREATE TABLE site_path_redirects (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  source_path VARCHAR(255) NOT NULL,
  target VARCHAR(500) NOT NULL,
  permanent BOOLEAN NOT NULL DEFAULT TRUE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY site_path_redirects_source_unique (source_path)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
