CREATE TABLE webhook_endpoints (
  id CHAR(36) NOT NULL,
  url VARCHAR(2048) NOT NULL,
  description VARCHAR(255) NULL,
  events JSON NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  disabled_reason VARCHAR(20) NULL,
  disabled_at DATETIME(6) NULL,
  consecutive_failures INT UNSIGNED NOT NULL DEFAULT 0,
  last_delivery_at DATETIME(6) NULL,
  last_success_at DATETIME(6) NULL,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY webhook_endpoints_is_active_index (is_active, created_at),
  CONSTRAINT webhook_endpoints_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE webhook_deliveries (
  id CHAR(36) NOT NULL,
  endpoint_id CHAR(36) NOT NULL,
  event_id CHAR(36) NOT NULL,
  event_type VARCHAR(100) NOT NULL,
  trigger_kind VARCHAR(20) NOT NULL,
  dedupe_key VARCHAR(100) NULL,
  redelivery_of CHAR(36) NULL,
  body MEDIUMTEXT NOT NULL,
  status VARCHAR(20) NOT NULL,
  attempts INT UNSIGNED NOT NULL DEFAULT 0,
  max_attempts INT UNSIGNED NOT NULL,
  next_attempt_at DATETIME(6) NULL,
  last_attempt_at DATETIME(6) NULL,
  response_status SMALLINT UNSIGNED NULL,
  response_body TEXT NULL,
  duration_ms INT UNSIGNED NULL,
  error VARCHAR(500) NULL,
  requested_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  completed_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY webhook_deliveries_dedupe_unique (dedupe_key),
  KEY webhook_deliveries_endpoint_index (endpoint_id, created_at),
  KEY webhook_deliveries_due_index (status, next_attempt_at),
  KEY webhook_deliveries_created_index (created_at),
  CONSTRAINT webhook_deliveries_endpoint_fk FOREIGN KEY (endpoint_id) REFERENCES webhook_endpoints (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE incoming_webhooks (
  id CHAR(36) NOT NULL,
  name VARCHAR(100) NOT NULL,
  description VARCHAR(500) NULL,
  token_prefix VARCHAR(16) NOT NULL,
  token_hash CHAR(64) NOT NULL,
  is_active TINYINT(1) NOT NULL DEFAULT 1,
  signature_required TINYINT(1) NOT NULL DEFAULT 0,
  signature_tolerance_seconds INT UNSIGNED NOT NULL DEFAULT 300,
  actions JSON NOT NULL,
  created_by CHAR(36) NOT NULL,
  last_received_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY incoming_webhooks_token_hash_unique (token_hash),
  KEY incoming_webhooks_created_at_index (created_at),
  KEY incoming_webhooks_created_by_index (created_by),
  CONSTRAINT incoming_webhooks_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE incoming_webhook_requests (
  id CHAR(36) NOT NULL,
  webhook_id CHAR(36) NOT NULL,
  received_at DATETIME(6) NOT NULL,
  ip VARCHAR(45) NULL,
  content_type VARCHAR(255) NULL,
  headers JSON NOT NULL,
  body MEDIUMTEXT NULL,
  body_bytes INT UNSIGNED NOT NULL DEFAULT 0,
  body_truncated TINYINT(1) NOT NULL DEFAULT 0,
  status VARCHAR(20) NOT NULL,
  http_status SMALLINT UNSIGNED NOT NULL,
  error_code VARCHAR(100) NULL,
  error_message VARCHAR(1000) NULL,
  results JSON NOT NULL,
  duration_ms INT UNSIGNED NOT NULL DEFAULT 0,
  PRIMARY KEY (id),
  KEY incoming_webhook_requests_webhook_index (webhook_id, received_at),
  KEY incoming_webhook_requests_received_index (received_at),
  CONSTRAINT incoming_webhook_requests_webhook_fk FOREIGN KEY (webhook_id) REFERENCES incoming_webhooks (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE incoming_webhook_replays (
  webhook_id CHAR(36) NOT NULL,
  replay_key CHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  seen_at DATETIME(6) NOT NULL,
  PRIMARY KEY (webhook_id, replay_key),
  KEY incoming_webhook_replays_seen_index (seen_at),
  CONSTRAINT incoming_webhook_replays_webhook_fk FOREIGN KEY (webhook_id) REFERENCES incoming_webhooks (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

