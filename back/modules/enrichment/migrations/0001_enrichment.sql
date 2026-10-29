CREATE TABLE enrichment_lookups (
  id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  cnpj VARCHAR(14) NOT NULL,
  source VARCHAR(32) NOT NULL,
  data JSON NOT NULL,
  fetched_by CHAR(36) NULL,
  fetched_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY enrichment_lookups_record_index (record_type, record_id, fetched_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE enrichment_field_sources (
  id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  field_key VARCHAR(64) NOT NULL,
  data_key VARCHAR(64) NOT NULL,
  source VARCHAR(32) NOT NULL,
  source_reference VARCHAR(32) NOT NULL,
  value_snapshot TEXT NULL,
  fetched_at DATETIME(6) NOT NULL,
  applied_by CHAR(36) NULL,
  applied_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY enrichment_field_sources_field_unique (record_type, record_id, field_key)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
