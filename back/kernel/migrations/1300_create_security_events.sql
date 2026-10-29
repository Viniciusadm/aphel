CREATE TABLE security_events (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NULL,
  kind VARCHAR(64) NOT NULL,
  subject VARCHAR(255) NULL,
  ip VARCHAR(45) NULL,
  user_agent VARCHAR(255) NULL,
  metadata JSON NOT NULL DEFAULT (JSON_OBJECT()),
  occurred_at DATETIME(6) NOT NULL,
  PRIMARY KEY (id),
  KEY security_events_user_index (user_id, occurred_at),
  KEY security_events_occurred_at_index (occurred_at),
  KEY security_events_subject_index (kind, subject, occurred_at),
  CONSTRAINT security_events_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
