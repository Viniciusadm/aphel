CREATE TABLE privacy_request_items (
  request_id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  processed_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (request_id, record_type, record_id),
  CONSTRAINT privacy_request_items_request_fk FOREIGN KEY (request_id) REFERENCES privacy_requests (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

ALTER TABLE retention_runs
  ADD COLUMN heartbeat_at DATETIME(6) NULL AFTER started_at,
  ADD KEY retention_runs_unfinished_index (status, heartbeat_at);
