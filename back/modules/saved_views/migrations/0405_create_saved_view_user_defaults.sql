CREATE TABLE saved_view_user_defaults (
  user_id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  view_id CHAR(36) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (user_id, record_type),
  KEY saved_view_user_defaults_view_index (view_id),
  CONSTRAINT saved_view_user_defaults_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT saved_view_user_defaults_view_fk FOREIGN KEY (view_id) REFERENCES saved_views (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
