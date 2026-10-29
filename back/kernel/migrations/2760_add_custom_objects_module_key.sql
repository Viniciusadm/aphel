ALTER TABLE custom_objects
  ADD COLUMN module_key VARCHAR(64) NULL AFTER is_active,
  ADD KEY custom_objects_module_key_index (module_key);
