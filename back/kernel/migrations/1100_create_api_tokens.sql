CREATE TABLE api_tokens (
  id CHAR(36) NOT NULL,
  created_by CHAR(36) NOT NULL,
  name VARCHAR(100) NOT NULL,
  token_prefix VARCHAR(16) NOT NULL,
  token_hash CHAR(64) NOT NULL,
  scopes JSON NOT NULL,
  expires_at DATETIME(6) NULL,
  last_used_at DATETIME(6) NULL,
  last_used_ip VARCHAR(45) NULL,
  revoked_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY api_tokens_token_hash_unique (token_hash),
  KEY api_tokens_created_at_index (created_at),
  KEY api_tokens_created_by_index (created_by),
  CONSTRAINT api_tokens_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
