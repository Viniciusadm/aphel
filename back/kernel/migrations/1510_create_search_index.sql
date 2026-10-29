CREATE TABLE search_index (
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  owner_id CHAR(36) NULL,
  title VARCHAR(255) NOT NULL,
  title_field VARCHAR(64) NULL,
  subtitle VARCHAR(255) NULL,
  subtitle_field VARCHAR(64) NULL,
  indexed_at DATETIME(6) NOT NULL,
  PRIMARY KEY (record_type, record_id),
  KEY search_index_owner_index (record_type, owner_id),
  KEY search_index_indexed_index (record_type, indexed_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
