UPDATE config_item_keys SET item_type = 'workflow.pipeline' WHERE item_type = 'pipeline';
UPDATE config_item_keys SET item_type = 'workflow.stage' WHERE item_type = 'pipeline_stage';
UPDATE config_item_keys SET item_type = 'workflow.lost_reason' WHERE item_type = 'lost_reason';
