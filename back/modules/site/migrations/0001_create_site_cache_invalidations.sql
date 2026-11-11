CREATE TABLE site_cache_invalidations (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  tag VARCHAR(191) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY site_cache_invalidations_created_at_index (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
