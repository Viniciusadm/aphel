CREATE TABLE user_tokens (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  purpose VARCHAR(30) NOT NULL,
  token_hash CHAR(64) NOT NULL,
  payload JSON NULL,
  expires_at DATETIME(6) NOT NULL,
  used_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY user_tokens_token_hash_unique (token_hash),
  KEY user_tokens_user_purpose_index (user_id, purpose),
  CONSTRAINT user_tokens_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
