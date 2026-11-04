CREATE TABLE google_calendar_connections (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  account_email VARCHAR(320) NULL,
  status VARCHAR(16) NOT NULL,
  sync_cursor TEXT NULL,
  last_synced_at DATETIME(6) NULL,
  last_error_code VARCHAR(64) NULL,
  last_error_message VARCHAR(500) NULL,
  imported_count INT UNSIGNED NOT NULL DEFAULT 0,
  exported_count INT UNSIGNED NOT NULL DEFAULT 0,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY google_calendar_connections_user_unique (user_id),
  KEY google_calendar_connections_status_index (status, last_synced_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE google_calendar_links (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  activity_id CHAR(36) NOT NULL,
  remote_id VARCHAR(255) NOT NULL,
  remote_etag VARCHAR(255) NULL,
  remote_updated_at DATETIME(6) NULL,
  local_updated_at DATETIME(6) NULL,
  last_synced_at DATETIME(6) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY google_calendar_links_remote_unique (user_id, remote_id),
  UNIQUE KEY google_calendar_links_activity_unique (activity_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
