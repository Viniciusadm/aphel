UPDATE config_item_keys SET item_type = 'views.view' WHERE item_type = 'saved_view';
ALTER TABLE field_definitions MODIFY field_type VARCHAR(100) NOT NULL;
ALTER TABLE domain_events MODIFY actor_type VARCHAR(100) NOT NULL;
ALTER TABLE audit_logs MODIFY actor_type VARCHAR(100) NOT NULL;
ALTER TABLE config_item_keys MODIFY item_type VARCHAR(100) NOT NULL;
