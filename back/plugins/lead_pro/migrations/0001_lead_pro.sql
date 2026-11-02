CREATE TABLE lead_pro_contact_extension (
  record_id CHAR(36) NOT NULL PRIMARY KEY,
  lead_temperature DECIMAL(24,6) NULL,
  KEY lead_pro_temperature_index (lead_temperature)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE lead_pro_touches (
  id CHAR(36) NOT NULL,
  contact_id CHAR(36) NOT NULL,
  kind VARCHAR(64) NOT NULL,
  event_id CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_lead_pro_touches_event (event_id),
  KEY idx_lead_pro_touches_contact (contact_id, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
