CREATE TABLE integrations (
  id CHAR(36) NOT NULL,
  provider_key VARCHAR(64) NOT NULL,
  config JSON NOT NULL,
  status VARCHAR(16) NOT NULL DEFAULT 'inactive',
  last_error VARCHAR(1000) NULL,
  last_error_at DATETIME(6) NULL,
  recent_errors JSON NOT NULL,
  last_tested_at DATETIME(6) NULL,
  last_test_ok TINYINT(1) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY integrations_provider_unique (provider_key)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
