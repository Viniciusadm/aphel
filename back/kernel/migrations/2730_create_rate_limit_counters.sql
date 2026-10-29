CREATE TABLE rate_limit_counters (
  bucket CHAR(64) NOT NULL,
  window_start BIGINT NOT NULL,
  hits INT UNSIGNED NOT NULL,
  expires_at DATETIME(3) NOT NULL,
  PRIMARY KEY (bucket, window_start),
  KEY rate_limit_counters_expires_index (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
