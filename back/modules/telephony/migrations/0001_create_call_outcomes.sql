CREATE TABLE call_outcomes (
  id CHAR(36) NOT NULL,
  `key` VARCHAR(64) NOT NULL,
  name VARCHAR(140) NOT NULL,
  is_connected BOOLEAN NOT NULL DEFAULT FALSE,
  is_system BOOLEAN NOT NULL DEFAULT FALSE,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  position INT NOT NULL DEFAULT 0,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY call_outcomes_key_unique (`key`),
  KEY call_outcomes_position_index (position)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
