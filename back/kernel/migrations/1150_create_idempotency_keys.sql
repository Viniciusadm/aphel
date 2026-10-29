CREATE TABLE idempotency_keys (
  scope_hash CHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  idempotency_key VARCHAR(255) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  request_hash CHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  response_status SMALLINT UNSIGNED NULL,
  response_content_type VARCHAR(255) NULL,
  response_body MEDIUMBLOB NULL,
  started_at DATETIME(6) NOT NULL,
  completed_at DATETIME(6) NULL,
  expires_at DATETIME(6) NOT NULL,
  PRIMARY KEY (scope_hash, idempotency_key),
  KEY idx_idempotency_keys_expires_at (expires_at)
);
