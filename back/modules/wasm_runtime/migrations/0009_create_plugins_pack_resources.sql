CREATE TABLE plugins_pack_resources (
  pack_key VARCHAR(48) NOT NULL,
  resource_key VARCHAR(80) NOT NULL,
  kind VARCHAR(64) NOT NULL,
  target VARCHAR(64) NULL,
  resource_id VARCHAR(64) NULL,
  mode VARCHAR(12) NOT NULL,
  data_hash CHAR(64) NOT NULL,
  applied JSON NULL,
  status VARCHAR(20) NOT NULL,
  error JSON NULL,
  applied_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (pack_key, resource_key)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
