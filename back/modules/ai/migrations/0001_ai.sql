CREATE TABLE ai_calls (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NULL,
  feature VARCHAR(32) NOT NULL,
  provider VARCHAR(32) NOT NULL,
  model VARCHAR(120) NOT NULL,
  status VARCHAR(16) NOT NULL,
  input_tokens INT UNSIGNED NULL,
  output_tokens INT UNSIGNED NULL,
  duration_ms INT UNSIGNED NOT NULL,
  error_code VARCHAR(64) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY ai_calls_created_at_index (created_at),
  KEY ai_calls_feature_index (feature, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE ai_usage (
  day DATE NOT NULL,
  calls INT UNSIGNED NOT NULL DEFAULT 0,
  PRIMARY KEY (day)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
