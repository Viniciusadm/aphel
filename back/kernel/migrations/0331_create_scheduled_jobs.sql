CREATE TABLE scheduled_jobs (
  id CHAR(36) NOT NULL,
  kind VARCHAR(150) NOT NULL,
  cron VARCHAR(255) NOT NULL,
  payload JSON NOT NULL,
  queue VARCHAR(64) NOT NULL,
  max_attempts INT UNSIGNED NOT NULL,
  enabled TINYINT(1) NOT NULL DEFAULT 1,
  last_run_at DATETIME(6) NULL,
  next_run_at DATETIME(6) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY scheduled_jobs_kind_unique (kind),
  KEY scheduled_jobs_due_index (enabled, next_run_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
