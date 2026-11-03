CREATE TABLE favorites (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  favorite_type VARCHAR(16) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NULL,
  view_id CHAR(36) NULL,
  target_id CHAR(36) NOT NULL,
  position INT NOT NULL DEFAULT 0,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY favorites_target_unique (user_id, favorite_type, record_type, target_id),
  KEY favorites_user_position_index (user_id, position),
  CONSTRAINT favorites_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
