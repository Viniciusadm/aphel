CREATE TABLE secrets (
  id CHAR(36) NOT NULL,
  owner_type VARCHAR(64) NOT NULL,
  owner_id VARCHAR(64) NOT NULL,
  name VARCHAR(64) NOT NULL,
  key_id CHAR(16) NOT NULL,
  nonce VARBINARY(12) NOT NULL,
  ciphertext BLOB NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY secrets_owner_unique (owner_type, owner_id, name),
  KEY secrets_key_id_index (key_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
