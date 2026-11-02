CREATE TABLE form_layouts (
  id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  context VARCHAR(16) NOT NULL,
  layout JSON NOT NULL,
  updated_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY form_layouts_context_unique (record_type, context),
  KEY form_layouts_updated_by_index (updated_by),
  CONSTRAINT form_layouts_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
