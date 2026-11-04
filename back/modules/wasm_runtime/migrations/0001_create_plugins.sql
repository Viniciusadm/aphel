CREATE TABLE plugins_installed (
  plugin_key VARCHAR(48) NOT NULL,
  name VARCHAR(120) NOT NULL,
  version VARCHAR(32) NOT NULL,
  description TEXT NULL,
  manifest JSON NOT NULL,
  checksum CHAR(64) NOT NULL,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  settings JSON NOT NULL DEFAULT (JSON_OBJECT()),
  installed_by CHAR(36) NULL,
  installed_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (plugin_key),
  KEY plugins_installed_installed_by_index (installed_by),
  CONSTRAINT plugins_installed_installed_by_fk FOREIGN KEY (installed_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE plugins_files (
  plugin_key VARCHAR(48) NOT NULL,
  path VARCHAR(255) NOT NULL,
  content_type VARCHAR(100) NOT NULL,
  content LONGBLOB NOT NULL,
  PRIMARY KEY (plugin_key, path),
  CONSTRAINT plugins_files_plugin_fk FOREIGN KEY (plugin_key) REFERENCES plugins_installed (plugin_key) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE plugins_data (
  plugin_key VARCHAR(48) NOT NULL,
  collection VARCHAR(64) NOT NULL,
  item_id VARCHAR(128) NOT NULL,
  value JSON NOT NULL,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (plugin_key, collection, item_id),
  CONSTRAINT plugins_data_plugin_fk FOREIGN KEY (plugin_key) REFERENCES plugins_installed (plugin_key) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE plugins_state (
  id TINYINT UNSIGNED NOT NULL,
  version BIGINT UNSIGNED NOT NULL DEFAULT 0,
  PRIMARY KEY (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT INTO plugins_state (id, version) VALUES (1, 0);
