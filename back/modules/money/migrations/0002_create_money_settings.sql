CREATE TABLE money_settings (
  id TINYINT NOT NULL,
  rates_provider VARCHAR(32) NOT NULL DEFAULT 'manual',
  rates_provider_url VARCHAR(255) NULL,
  rates_base CHAR(3) NULL,
  last_synced_at DATETIME(6) NULL,
  updated_by CHAR(36) NULL,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY money_settings_updated_by_index (updated_by),
  CONSTRAINT money_settings_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT INTO money_settings (id, rates_base)
SELECT 1, currency FROM organization_settings WHERE id = 1;
