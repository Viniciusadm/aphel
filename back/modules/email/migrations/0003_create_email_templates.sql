CREATE TABLE email_templates (
  id CHAR(36) NOT NULL,
  `key` VARCHAR(64) NOT NULL,
  name VARCHAR(200) NOT NULL,
  scope VARCHAR(16) NOT NULL,
  owner_id CHAR(36) NULL,
  owner_key CHAR(36) NOT NULL DEFAULT '',
  record_type VARCHAR(64) NULL,
  subject VARCHAR(1000) NOT NULL DEFAULT '',
  body_html MEDIUMTEXT NOT NULL,
  created_by CHAR(36) NULL,
  updated_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY email_templates_key_unique (owner_key, `key`),
  KEY email_templates_scope_index (scope, record_type),
  KEY email_templates_owner_fk_index (owner_id),
  KEY email_templates_created_by_fk_index (created_by),
  KEY email_templates_updated_by_fk_index (updated_by),
  CONSTRAINT email_templates_owner_fk FOREIGN KEY (owner_id) REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT email_templates_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL,
  CONSTRAINT email_templates_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE email_tracking_settings (
  id TINYINT UNSIGNED NOT NULL,
  opens_enabled TINYINT(1) NOT NULL DEFAULT 0,
  clicks_enabled TINYINT(1) NOT NULL DEFAULT 0,
  default_on TINYINT(1) NOT NULL DEFAULT 0,
  updated_by CHAR(36) NULL,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  CONSTRAINT email_tracking_settings_single_row CHECK (id = 1),
  KEY email_tracking_settings_updated_by_fk_index (updated_by),
  CONSTRAINT email_tracking_settings_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE email_tracked_links (
  id CHAR(36) NOT NULL,
  message_id CHAR(36) NOT NULL,
  position INT NOT NULL,
  url TEXT NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY email_tracked_links_position_unique (message_id, position),
  CONSTRAINT email_tracked_links_message_fk FOREIGN KEY (message_id) REFERENCES email_messages (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE email_tracking_objections (
  id CHAR(36) NOT NULL,
  address_hash CHAR(64) NOT NULL,
  label VARCHAR(320) NOT NULL,
  reason VARCHAR(500) NULL,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY email_tracking_objections_address_unique (address_hash),
  KEY email_tracking_objections_created_by_fk_index (created_by),
  CONSTRAINT email_tracking_objections_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
