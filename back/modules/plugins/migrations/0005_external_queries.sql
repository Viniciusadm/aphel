CREATE TABLE plugins_external_queries (
  plugin_key VARCHAR(48) NOT NULL,
  execution_id CHAR(36) NOT NULL,
  operation VARCHAR(128) NOT NULL,
  input_hash CHAR(64) NOT NULL,
  authority_hash CHAR(64) NOT NULL,
  status VARCHAR(16) NOT NULL DEFAULT 'running',
  output JSON NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  completed_at DATETIME(6) NULL,
  PRIMARY KEY (plugin_key, execution_id, operation, input_hash),
  CONSTRAINT plugins_external_queries_plugin_fk FOREIGN KEY (plugin_key) REFERENCES plugins_installed (plugin_key) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
