CREATE TABLE duplicate_rules (
  record_type VARCHAR(64) NOT NULL,
  rule_key VARCHAR(64) NOT NULL,
  is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (record_type, rule_key)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE duplicate_candidates (
  id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  duplicate_id CHAR(36) NOT NULL,
  rules JSON NOT NULL,
  status VARCHAR(16) NOT NULL DEFAULT 'pending',
  detected_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  resolved_at DATETIME(6) NULL,
  resolved_by CHAR(36) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY duplicate_candidates_pair_unique (record_type, record_id, duplicate_id),
  KEY duplicate_candidates_status_index (record_type, status, detected_at),
  KEY duplicate_candidates_duplicate_index (record_type, duplicate_id),
  CONSTRAINT duplicate_candidates_resolved_by_fk FOREIGN KEY (resolved_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE record_merges (
  record_type VARCHAR(64) NOT NULL,
  merged_id CHAR(36) NOT NULL,
  survivor_id CHAR(36) NOT NULL,
  merged_by CHAR(36) NULL,
  merged_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (record_type, merged_id),
  KEY record_merges_survivor_index (record_type, survivor_id),
  CONSTRAINT record_merges_merged_by_fk FOREIGN KEY (merged_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
