CREATE TABLE system_audit_logs (
  id CHAR(36) NOT NULL,
  action VARCHAR(100) NOT NULL,
  actor_user_id CHAR(36) NULL,
  subject_type VARCHAR(64) NULL,
  subject_id VARCHAR(64) NULL,
  metadata JSON NOT NULL,
  ip VARCHAR(45) NULL,
  occurred_at DATETIME(6) NOT NULL,
  PRIMARY KEY (id),
  KEY system_audit_logs_occurred_index (occurred_at),
  KEY system_audit_logs_actor_index (actor_user_id, occurred_at),
  CONSTRAINT system_audit_logs_actor_fk FOREIGN KEY (actor_user_id) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
