CREATE TABLE IF NOT EXISTS lead_pro_contact_extension (
  record_id CHAR(36) NOT NULL PRIMARY KEY,
  lead_temperature DECIMAL(24,6) NULL,
  KEY lead_pro_temperature_index (lead_temperature)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

SET @copy_temperature = IF((SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'people' AND COLUMN_NAME = 'lead_temperature') > 0, 'INSERT IGNORE INTO lead_pro_contact_extension (record_id, lead_temperature) SELECT id, lead_temperature FROM people WHERE lead_temperature IS NOT NULL', 'SELECT 1');
PREPARE copy_temperature FROM @copy_temperature;
EXECUTE copy_temperature;
DEALLOCATE PREPARE copy_temperature;
