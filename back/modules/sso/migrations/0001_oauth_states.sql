CREATE TABLE sso_oauth_states (
  id CHAR(36) NOT NULL,
  provider_key VARCHAR(64) NOT NULL,
  purpose VARCHAR(64) NOT NULL,
  state_hash CHAR(64) NOT NULL,
  nonce VARCHAR(128) NOT NULL,
  code_verifier VARCHAR(128) NOT NULL,
  user_id CHAR(36) NULL,
  redirect_path VARCHAR(512) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  expires_at DATETIME(6) NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY sso_oauth_states_hash_unique (provider_key, state_hash),
  KEY sso_oauth_states_expires_index (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
