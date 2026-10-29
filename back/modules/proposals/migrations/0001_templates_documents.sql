CREATE TABLE proposals_templates (
  id CHAR(36) NOT NULL,
  template_key VARCHAR(64) NOT NULL,
  name VARCHAR(160) NOT NULL,
  description VARCHAR(500) NULL,
  body MEDIUMTEXT NOT NULL,
  is_active TINYINT(1) NOT NULL DEFAULT 1,
  is_default TINYINT(1) NOT NULL DEFAULT 0,
  created_by CHAR(36) NULL,
  updated_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY proposals_templates_key_unique (template_key),
  KEY proposals_templates_name_index (name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE proposals_counters (
  id TINYINT UNSIGNED NOT NULL,
  last_number INT UNSIGNED NOT NULL,
  PRIMARY KEY (id),
  CONSTRAINT proposals_counters_single_row CHECK (id = 1)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE proposals_documents (
  id CHAR(36) NOT NULL,
  deal_id CHAR(36) NOT NULL,
  template_id CHAR(36) NULL,
  template_name VARCHAR(160) NOT NULL,
  number INT UNSIGNED NOT NULL,
  file_id CHAR(36) NOT NULL,
  file_name VARCHAR(255) NOT NULL,
  file_size BIGINT UNSIGNED NOT NULL,
  generated_by CHAR(36) NULL,
  generated_by_name VARCHAR(200) NULL,
  actor_type VARCHAR(32) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY proposals_documents_number_unique (number),
  KEY proposals_documents_deal_index (deal_id, created_at),
  KEY proposals_documents_template_index (template_id),
  CONSTRAINT proposals_documents_template_fk FOREIGN KEY (template_id) REFERENCES proposals_templates (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
