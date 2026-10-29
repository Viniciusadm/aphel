DROP TABLE IF EXISTS lead_pro_touches;

ALTER TABLE contacts
  DROP INDEX idx_contacts_lead_temperature,
  DROP COLUMN lead_temperature;
