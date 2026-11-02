CREATE TABLE notification_preferences (
  user_id CHAR(36) NOT NULL,
  kind VARCHAR(64) NOT NULL,
  channel VARCHAR(16) NOT NULL,
  enabled TINYINT(1) NOT NULL,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (user_id, kind, channel),
  KEY notification_preferences_kind_index (kind, channel, enabled),
  CONSTRAINT notification_preferences_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
