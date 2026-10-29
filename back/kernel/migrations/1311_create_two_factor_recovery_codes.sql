CREATE TABLE two_factor_recovery_codes (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  code_hash CHAR(64) NOT NULL,
  used_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY two_factor_recovery_codes_unique (user_id, code_hash),
  CONSTRAINT two_factor_recovery_codes_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
