CREATE TABLE audit_logs (
  id CHAR(36) NOT NULL,
  action VARCHAR(150) NOT NULL,
  record_type VARCHAR(64) NULL,
  record_id CHAR(36) NULL,
  actor_type VARCHAR(20) NOT NULL,
  actor_id VARCHAR(64) NULL,
  changes JSON NOT NULL,
  ip VARCHAR(45) NULL,
  occurred_at DATETIME(6) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY audit_logs_record_index (record_type, record_id, occurred_at),
  KEY audit_logs_occurred_index (occurred_at),
  KEY audit_logs_actor_index (actor_type, actor_id, occurred_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
