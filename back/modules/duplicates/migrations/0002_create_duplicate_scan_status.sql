CREATE TABLE duplicate_scan_status (
  id TINYINT NOT NULL,
  status VARCHAR(16) NOT NULL,
  requested_by CHAR(36) NULL,
  requested_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  started_at DATETIME(6) NULL,
  finished_at DATETIME(6) NULL,
  records INT UNSIGNED NOT NULL DEFAULT 0,
  candidates INT UNSIGNED NOT NULL DEFAULT 0,
  PRIMARY KEY (id),
  CONSTRAINT duplicate_scan_status_single_row CHECK (id = 1)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
