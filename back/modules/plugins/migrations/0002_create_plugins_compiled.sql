CREATE TABLE plugins_compiled (
  backend_hash CHAR(64) NOT NULL,
  engine_key CHAR(16) NOT NULL,
  artifact LONGBLOB NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (backend_hash, engine_key)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
