CREATE TABLE system_heartbeats (
  instance_id VARCHAR(128) NOT NULL,
  role VARCHAR(32) NOT NULL,
  hostname VARCHAR(255) NULL,
  version VARCHAR(64) NOT NULL,
  started_at DATETIME(6) NOT NULL,
  last_seen_at DATETIME(6) NOT NULL,
  PRIMARY KEY (instance_id),
  KEY system_heartbeats_role_index (role, last_seen_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
