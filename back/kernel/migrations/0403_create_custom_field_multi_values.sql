CREATE TABLE custom_field_multi_values (
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  field_id CHAR(36) NOT NULL,
  option_id CHAR(36) NOT NULL,
  PRIMARY KEY (record_type, record_id, field_id, option_id),
  KEY custom_field_multi_values_option_index (field_id, option_id),
  CONSTRAINT custom_field_multi_values_field_fk FOREIGN KEY (field_id) REFERENCES field_definitions (id) ON DELETE CASCADE,
  CONSTRAINT custom_field_multi_values_option_fk FOREIGN KEY (option_id) REFERENCES field_options (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
