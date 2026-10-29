CREATE TABLE field_options (
  id CHAR(36) NOT NULL,
  field_id CHAR(36) NOT NULL,
  value VARCHAR(64) NOT NULL,
  label VARCHAR(140) NOT NULL,
  color VARCHAR(7) NULL,
  position INT NOT NULL DEFAULT 0,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY field_options_value_unique (field_id, value),
  KEY field_options_field_id_index (field_id, position),
  CONSTRAINT field_options_field_fk FOREIGN KEY (field_id) REFERENCES field_definitions (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
