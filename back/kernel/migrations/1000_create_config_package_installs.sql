CREATE TABLE config_package_installs (
  id CHAR(36) NOT NULL,
  name VARCHAR(140) NOT NULL,
  format_version INT NOT NULL,
  template_key VARCHAR(64) NULL,
  package JSON NOT NULL,
  summary JSON NOT NULL,
  installed_by CHAR(36) NULL,
  installed_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY config_package_installs_installed_at_index (installed_at),
  KEY config_package_installs_installed_by_index (installed_by),
  CONSTRAINT config_package_installs_installed_by_fk FOREIGN KEY (installed_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
