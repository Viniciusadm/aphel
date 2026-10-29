CREATE TABLE segments (
  id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  name VARCHAR(140) NOT NULL,
  description VARCHAR(500) NULL,
  filter JSON NOT NULL,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY segments_name_unique (record_type, name),
  KEY segments_created_by_index (created_by),
  CONSTRAINT segments_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
