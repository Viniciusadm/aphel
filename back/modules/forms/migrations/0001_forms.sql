CREATE TABLE forms_forms (
  id CHAR(36) NOT NULL,
  public_id VARCHAR(64) NOT NULL,
  name VARCHAR(120) NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  acting_user_id CHAR(36) NULL,
  definition JSON NOT NULL,
  created_by CHAR(36) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY forms_forms_public_id_unique (public_id),
  KEY forms_forms_created_at_index (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE forms_submissions (
  id CHAR(36) NOT NULL,
  form_id CHAR(36) NOT NULL,
  status VARCHAR(16) NOT NULL,
  contact_id CHAR(36) NULL,
  deal_id CHAR(36) NULL,
  contact_created BOOLEAN NOT NULL DEFAULT FALSE,
  data JSON NOT NULL,
  ip_address VARCHAR(64) NULL,
  origin VARCHAR(255) NULL,
  error_code VARCHAR(64) NULL,
  error_message VARCHAR(500) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY forms_submissions_form_index (form_id, created_at),
  KEY forms_submissions_status_index (form_id, status),
  CONSTRAINT forms_submissions_form_fk FOREIGN KEY (form_id) REFERENCES forms_forms (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
