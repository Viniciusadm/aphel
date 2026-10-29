CREATE TABLE organization_settings (
  id TINYINT UNSIGNED NOT NULL,
  name VARCHAR(140) NOT NULL,
  logo_file_id CHAR(36) NULL,
  primary_color VARCHAR(7) NULL,
  locale VARCHAR(10) NOT NULL DEFAULT 'pt-BR',
  timezone VARCHAR(64) NOT NULL DEFAULT 'America/Sao_Paulo',
  currency CHAR(3) NOT NULL DEFAULT 'BRL',
  settings JSON NOT NULL DEFAULT (JSON_OBJECT()),
  custom_schema_version BIGINT UNSIGNED NOT NULL DEFAULT 0,
  two_factor_policy VARCHAR(16) NOT NULL DEFAULT 'none',
  installed_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  CONSTRAINT organization_settings_single_row CHECK (id = 1)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
