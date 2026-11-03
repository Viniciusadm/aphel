CREATE TABLE pipeline_renewal_settings (
  pipeline_id CHAR(36) NOT NULL,
  is_enabled BOOLEAN NOT NULL DEFAULT FALSE,
  days_before SMALLINT UNSIGNED NOT NULL DEFAULT 30,
  target_pipeline_id CHAR(36) NOT NULL,
  target_stage_id CHAR(36) NOT NULL,
  title_prefix VARCHAR(60) NOT NULL DEFAULT 'Renovação: ',
  owner_mode VARCHAR(16) NOT NULL DEFAULT 'original',
  updated_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (pipeline_id),
  KEY pipeline_renewal_settings_enabled_index (is_enabled),
  KEY pipeline_renewal_settings_target_pipeline_fk_index (target_pipeline_id),
  KEY pipeline_renewal_settings_target_stage_fk_index (target_stage_id),
  KEY pipeline_renewal_settings_updated_by_fk_index (updated_by),
  CONSTRAINT pipeline_renewal_settings_pipeline_fk FOREIGN KEY (pipeline_id) REFERENCES workflow (id) ON DELETE CASCADE,
  CONSTRAINT pipeline_renewal_settings_target_pipeline_fk FOREIGN KEY (target_pipeline_id) REFERENCES workflow (id) ON DELETE CASCADE,
  CONSTRAINT pipeline_renewal_settings_target_stage_fk FOREIGN KEY (target_stage_id) REFERENCES workflow_stages (id) ON DELETE CASCADE,
  CONSTRAINT pipeline_renewal_settings_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;


CREATE TABLE deal_renewals (
  original_deal_id CHAR(36) NOT NULL,
  renewal_deal_id CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (original_deal_id),
  KEY deal_renewals_renewal_fk_index (renewal_deal_id),
  CONSTRAINT deal_renewals_original_fk FOREIGN KEY (original_deal_id) REFERENCES deals (id) ON DELETE CASCADE,
  CONSTRAINT deal_renewals_renewal_fk FOREIGN KEY (renewal_deal_id) REFERENCES deals (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
