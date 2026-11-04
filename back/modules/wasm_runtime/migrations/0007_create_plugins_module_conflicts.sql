CREATE TABLE plugins_module_conflicts (
  plugin_key VARCHAR(48) NOT NULL,
  module_key VARCHAR(64) NOT NULL,
  was_enabled BOOLEAN NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (plugin_key, module_key),
  KEY plugins_module_conflicts_module_index (module_key),
  CONSTRAINT plugins_module_conflicts_plugin_fk FOREIGN KEY (plugin_key) REFERENCES plugins_installed (plugin_key) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
