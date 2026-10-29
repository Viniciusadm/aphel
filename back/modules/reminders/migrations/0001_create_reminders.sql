CREATE TABLE reminders (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NULL,
  record_id CHAR(36) NULL,
  remind_at DATETIME(6) NOT NULL,
  note VARCHAR(500) NULL,
  status VARCHAR(16) NOT NULL DEFAULT 'pending',
  sent_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY reminders_user_index (user_id, status, remind_at),
  KEY reminders_due_index (status, remind_at),
  KEY reminders_record_index (record_type, record_id),
  CONSTRAINT reminders_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
