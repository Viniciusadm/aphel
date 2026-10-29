CREATE TABLE email_blocklist (
  id CHAR(36) NOT NULL,
  kind VARCHAR(16) NOT NULL,
  value VARCHAR(320) NOT NULL,
  reason VARCHAR(500) NULL,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY email_blocklist_value_unique (kind, value),
  KEY email_blocklist_created_by_fk_index (created_by),
  CONSTRAINT email_blocklist_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE email_capture_settings (
  id TINYINT UNSIGNED NOT NULL,
  create_contacts TINYINT(1) NOT NULL DEFAULT 0,
  create_companies TINYINT(1) NOT NULL DEFAULT 0,
  link_open_deals TINYINT(1) NOT NULL DEFAULT 1,
  initial_history_days INT NOT NULL DEFAULT 90,
  updated_by CHAR(36) NULL,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  CONSTRAINT email_capture_settings_single_row CHECK (id = 1),
  KEY email_capture_settings_updated_by_fk_index (updated_by),
  CONSTRAINT email_capture_settings_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
