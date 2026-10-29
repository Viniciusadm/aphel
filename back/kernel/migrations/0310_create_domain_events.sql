CREATE TABLE domain_events (
  id CHAR(36) NOT NULL,
  event_type VARCHAR(150) NOT NULL,
  record_type VARCHAR(64) NULL,
  record_id CHAR(36) NULL,
  actor_type VARCHAR(20) NOT NULL,
  actor_id VARCHAR(64) NULL,
  payload JSON NOT NULL,
  occurred_at DATETIME(6) NOT NULL,
  dispatched_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY domain_events_record_index (record_type, record_id, occurred_at),
  KEY domain_events_outbox_index (dispatched_at, occurred_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
