CREATE TABLE bulk_operations (
  id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  action VARCHAR(32) NOT NULL,
  params JSON NOT NULL,
  selection JSON NOT NULL,
  status VARCHAR(16) NOT NULL,
  total INT UNSIGNED NOT NULL DEFAULT 0,
  processed INT UNSIGNED NOT NULL DEFAULT 0,
  succeeded INT UNSIGNED NOT NULL DEFAULT 0,
  skipped INT UNSIGNED NOT NULL DEFAULT 0,
  failed INT UNSIGNED NOT NULL DEFAULT 0,
  failure_reason VARCHAR(500) NULL,
  requested_by CHAR(36) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  finished_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  KEY bulk_operations_requested_by_index (requested_by, created_at),
  CONSTRAINT bulk_operations_requested_by_fk FOREIGN KEY (requested_by) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE bulk_operation_items (
  operation_id CHAR(36) NOT NULL,
  record_id CHAR(36) NOT NULL,
  outcome VARCHAR(16) NOT NULL,
  code VARCHAR(64) NULL,
  message VARCHAR(500) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (operation_id, record_id),
  KEY bulk_operation_items_outcome_index (operation_id, outcome),
  CONSTRAINT bulk_operation_items_operation_fk FOREIGN KEY (operation_id) REFERENCES bulk_operations (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
