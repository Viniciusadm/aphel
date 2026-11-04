CREATE TABLE plugins_changes (
  seq BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  plugin_key VARCHAR(48) NOT NULL,
  event_id CHAR(36) NOT NULL,
  event_type VARCHAR(150) NOT NULL,
  record_type VARCHAR(64) NULL,
  record_id CHAR(36) NULL,
  fields JSON NOT NULL,
  occurred_at DATETIME(6) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (seq),
  UNIQUE KEY plugins_changes_event_unique (plugin_key, event_id),
  KEY plugins_changes_plugin_index (plugin_key, seq),
  KEY plugins_changes_created_index (created_at),
  CONSTRAINT plugins_changes_plugin_fk FOREIGN KEY (plugin_key) REFERENCES plugins_installed (plugin_key) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
