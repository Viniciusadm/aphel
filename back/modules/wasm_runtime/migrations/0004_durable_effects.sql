CREATE TABLE plugins_effects (
  id CHAR(36) NOT NULL,
  plugin_key VARCHAR(48) NOT NULL,
  execution_id CHAR(36) NOT NULL,
  effect_key VARCHAR(128) NOT NULL,
  command JSON NOT NULL,
  authority JSON NOT NULL,
  execution JSON NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'pending',
  result JSON NULL,
  last_error TEXT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  completed_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY plugins_effects_execution_key (plugin_key, execution_id, effect_key),
  KEY plugins_effects_status (status, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE plugins_executions (
  id CHAR(36) NOT NULL,
  plugin_key VARCHAR(48) NOT NULL,
  source_key VARCHAR(160) NOT NULL,
  input JSON NOT NULL,
  execution JSON NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'pending',
  last_error TEXT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  completed_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY plugins_executions_source (plugin_key, source_key)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE plugins_effect_limits (
  plugin_key VARCHAR(48) NOT NULL,
  execution_id CHAR(36) NOT NULL,
  effect_count INT UNSIGNED NOT NULL DEFAULT 0,
  PRIMARY KEY (plugin_key, execution_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
