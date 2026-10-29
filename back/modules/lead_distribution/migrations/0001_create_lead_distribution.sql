CREATE TABLE lead_distribution_rules (
  id CHAR(36) NOT NULL,
  `key` VARCHAR(64) NOT NULL,
  name VARCHAR(140) NOT NULL,
  description TEXT NULL,
  record_type VARCHAR(64) NOT NULL,
  strategy VARCHAR(20) NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  routing_field VARCHAR(64) NULL,
  routes JSON NULL,
  routing_fallback VARCHAR(16) NOT NULL DEFAULT 'members',
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY lead_distribution_rules_key_unique (`key`),
  KEY lead_distribution_rules_record_type_index (record_type, is_active),
  KEY lead_distribution_rules_created_by_fk_index (created_by),
  CONSTRAINT lead_distribution_rules_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE lead_distribution_members (
  rule_id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  position INT UNSIGNED NOT NULL,
  weight INT UNSIGNED NOT NULL DEFAULT 1,
  is_away BOOLEAN NOT NULL DEFAULT FALSE,
  away_from DATETIME(6) NULL,
  away_until DATETIME(6) NULL,
  PRIMARY KEY (rule_id, user_id),
  KEY lead_distribution_members_user_index (user_id),
  CONSTRAINT lead_distribution_members_rule_fk FOREIGN KEY (rule_id) REFERENCES lead_distribution_rules (id) ON DELETE CASCADE,
  CONSTRAINT lead_distribution_members_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE lead_distribution_states (
  rule_id CHAR(36) NOT NULL,
  sequence BIGINT UNSIGNED NOT NULL DEFAULT 0,
  last_user_id CHAR(36) NULL,
  last_assigned_at DATETIME(6) NULL,
  PRIMARY KEY (rule_id),
  CONSTRAINT lead_distribution_states_rule_fk FOREIGN KEY (rule_id) REFERENCES lead_distribution_rules (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE lead_distribution_counters (
  rule_id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  assigned BIGINT UNSIGNED NOT NULL DEFAULT 0,
  last_sequence BIGINT UNSIGNED NOT NULL DEFAULT 0,
  credit BIGINT NOT NULL DEFAULT 0,
  last_assigned_at DATETIME(6) NULL,
  PRIMARY KEY (rule_id, user_id),
  KEY lead_distribution_counters_user_index (user_id),
  CONSTRAINT lead_distribution_counters_rule_fk FOREIGN KEY (rule_id) REFERENCES lead_distribution_rules (id) ON DELETE CASCADE,
  CONSTRAINT lead_distribution_counters_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE lead_distribution_assignments (
  id CHAR(36) NOT NULL,
  rule_id CHAR(36) NOT NULL,
  user_id CHAR(36) NULL,
  record_type VARCHAR(64) NULL,
  record_id CHAR(36) NULL,
  source VARCHAR(20) NOT NULL,
  idempotency_key VARCHAR(191) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY lead_distribution_assignments_key_unique (idempotency_key),
  KEY lead_distribution_assignments_rule_index (rule_id, created_at),
  KEY lead_distribution_assignments_user_fk_index (user_id),
  CONSTRAINT lead_distribution_assignments_rule_fk FOREIGN KEY (rule_id) REFERENCES lead_distribution_rules (id) ON DELETE CASCADE,
  CONSTRAINT lead_distribution_assignments_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
