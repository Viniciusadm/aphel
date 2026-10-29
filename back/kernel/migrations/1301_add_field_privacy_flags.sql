ALTER TABLE field_definitions
  ADD COLUMN is_personal BOOLEAN NOT NULL DEFAULT FALSE AFTER is_searchable,
  ADD COLUMN is_sensitive BOOLEAN NOT NULL DEFAULT FALSE AFTER is_personal;

UPDATE field_definitions
SET is_personal = TRUE
WHERE is_system
  AND record_type = 'contact'
  AND `key` IN (
    'name', 'first_name', 'last_name', 'email', 'phone', 'mobile',
    'address_street', 'address_number', 'address_complement', 'address_district',
    'address_city', 'address_state', 'address_postal_code', 'address_country'
  );
