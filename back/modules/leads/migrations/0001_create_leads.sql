CREATE TABLE leads_contact_extension (
  record_id CHAR(36) NOT NULL PRIMARY KEY,
  lifecycle_stage VARCHAR(64) NULL,
  KEY leads_contact_stage_index (lifecycle_stage)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE leads_company_extension (
  record_id CHAR(36) NOT NULL PRIMARY KEY,
  lifecycle_stage VARCHAR(64) NULL,
  KEY leads_company_stage_index (lifecycle_stage)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE lifecycle_stage_history (
  id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  from_stage VARCHAR(64) NULL,
  to_stage VARCHAR(64) NOT NULL,
  entered_at DATETIME(6) NOT NULL,
  left_at DATETIME(6) NULL,
  actor_type VARCHAR(32) NOT NULL,
  actor_id VARCHAR(64) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY lifecycle_stage_history_record_index (record_type, record_id, entered_at),
  KEY lifecycle_stage_history_open_index (record_type, record_id, left_at),
  KEY lifecycle_stage_history_period_index (record_type, entered_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE lifecycle_settings (
  id TINYINT UNSIGNED NOT NULL,
  rules JSON NOT NULL,
  disqualified_stage VARCHAR(64) NULL,
  updated_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  CONSTRAINT lifecycle_settings_single_row CHECK (id = 1)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE lead_disqualification_reasons (
  id CHAR(36) NOT NULL,
  `key` VARCHAR(64) NOT NULL,
  name VARCHAR(140) NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  position INT NOT NULL DEFAULT 0,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY lead_disqualification_reasons_key_unique (`key`),
  KEY lead_disqualification_reasons_position_index (position)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE lead_disqualifications (
  id CHAR(36) NOT NULL,
  contact_id CHAR(36) NOT NULL,
  reason_id CHAR(36) NULL,
  reason_name VARCHAR(140) NOT NULL,
  note TEXT NULL,
  from_stage VARCHAR(64) NULL,
  actor_type VARCHAR(32) NOT NULL,
  actor_id VARCHAR(64) NULL,
  created_at DATETIME(6) NOT NULL,
  PRIMARY KEY (id),
  KEY lead_disqualifications_contact_index (contact_id, created_at),
  KEY lead_disqualifications_period_index (created_at),
  KEY lead_disqualifications_reason_fk_index (reason_id),
  CONSTRAINT lead_disqualifications_reason_fk FOREIGN KEY (reason_id) REFERENCES lead_disqualification_reasons (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
