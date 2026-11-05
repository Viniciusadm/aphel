CREATE TABLE activity_calls (
  activity_id CHAR(36) NOT NULL,
  direction VARCHAR(16) NOT NULL,
  outcome_key VARCHAR(64) NOT NULL,
  duration_seconds INT UNSIGNED NULL,
  phone_number VARCHAR(64) NULL,
  recording_url VARCHAR(2048) NULL,
  provider VARCHAR(128) NULL,
  provider_call_id VARCHAR(255) NULL,
  session_id CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (activity_id),
  UNIQUE KEY activity_calls_provider_unique (provider, provider_call_id),
  KEY activity_calls_outcome_index (outcome_key),
  KEY activity_calls_session_index (session_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
