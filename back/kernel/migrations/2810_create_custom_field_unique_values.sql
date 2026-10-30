CREATE TABLE custom_field_unique_values (
  field_id CHAR(36) NOT NULL,
  value_key CHAR(64) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (field_id, value_key),
  UNIQUE KEY custom_field_unique_values_record (record_type, record_id, field_id),
  CONSTRAINT custom_field_unique_values_field_fk FOREIGN KEY (field_id) REFERENCES field_definitions (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT IGNORE INTO custom_field_unique_values (field_id, value_key, record_type, record_id)
SELECT v.field_id,
       CASE WHEN v.value_text IS NOT NULL THEN SHA2(CONCAT('t:', LOWER(v.value_text)), 256)
            ELSE SHA2(CONCAT('n:', CAST(v.value_number AS CHAR)), 256) END,
       v.record_type,
       v.record_id
FROM custom_field_values v
JOIN field_definitions f ON f.id = v.field_id
WHERE f.is_unique = 1
  AND f.field_type IN ('text', 'number', 'url', 'email', 'phone')
  AND (v.value_text IS NOT NULL OR v.value_number IS NOT NULL);
