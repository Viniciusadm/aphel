CREATE TABLE plugins_packs (
  pack_key VARCHAR(48) NOT NULL,
  name VARCHAR(120) NOT NULL,
  version VARCHAR(32) NOT NULL,
  description TEXT NULL,
  manifest JSON NOT NULL,
  installed_by CHAR(36) NULL,
  installed_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (pack_key),
  KEY plugins_packs_installed_by_index (installed_by),
  CONSTRAINT plugins_packs_installed_by_fk FOREIGN KEY (installed_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

ALTER TABLE plugins_installed
  ADD COLUMN pack_key VARCHAR(48) NULL,
  ADD KEY plugins_installed_pack_index (pack_key);
