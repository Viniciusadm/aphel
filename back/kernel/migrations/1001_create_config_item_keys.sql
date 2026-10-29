CREATE TABLE config_item_keys (
  item_type VARCHAR(64) NOT NULL,
  item_id CHAR(36) NOT NULL,
  `key` VARCHAR(64) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (item_type, item_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
