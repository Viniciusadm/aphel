ALTER TABLE contacts
  ADD COLUMN lead_temperature TINYINT UNSIGNED NULL,
  ADD INDEX idx_contacts_lead_temperature (lead_temperature);

CREATE TABLE lead_pro_touches (
  id CHAR(36) NOT NULL,
  contact_id CHAR(36) NOT NULL,
  kind VARCHAR(64) NOT NULL,
  event_id CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_lead_pro_touches_event (event_id),
  KEY idx_lead_pro_touches_contact (contact_id, created_at),
  CONSTRAINT fk_lead_pro_touches_contact FOREIGN KEY (contact_id) REFERENCES contacts (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
