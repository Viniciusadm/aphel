CREATE TABLE forecast_settings (
  id TINYINT UNSIGNED NOT NULL,
  method VARCHAR(32) NOT NULL DEFAULT 'deal_probability',
  categories_enabled BOOLEAN NOT NULL DEFAULT FALSE,
  best_case_min_probability TINYINT UNSIGNED NOT NULL DEFAULT 50,
  commit_min_probability TINYINT UNSIGNED NOT NULL DEFAULT 90,
  horizon_months TINYINT UNSIGNED NOT NULL DEFAULT 6,
  updated_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  CONSTRAINT forecast_settings_single_row CHECK (id = 1),
  KEY forecast_settings_updated_by_fk_index (updated_by),
  CONSTRAINT forecast_settings_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
