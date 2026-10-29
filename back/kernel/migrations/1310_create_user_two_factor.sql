CREATE TABLE user_two_factor (
  user_id CHAR(36) NOT NULL,
  key_id CHAR(16) NOT NULL,
  nonce VARBINARY(12) NOT NULL,
  ciphertext BLOB NOT NULL,
  enabled_at DATETIME(6) NULL,
  last_used_step BIGINT UNSIGNED NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (user_id),
  KEY user_two_factor_key_id_index (key_id),
  CONSTRAINT user_two_factor_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
