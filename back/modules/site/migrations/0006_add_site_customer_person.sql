ALTER TABLE site_customers
  ADD COLUMN person_id CHAR(36) NULL AFTER name,
  ADD KEY site_customers_person_index (person_id);
