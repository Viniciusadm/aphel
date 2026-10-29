CREATE TABLE config_package_versions (
  id CHAR(36) NOT NULL,
  package_id VARCHAR(128) NOT NULL,
  version VARCHAR(64) NOT NULL,
  name VARCHAR(140) NOT NULL,
  install_id CHAR(36) NOT NULL,
  origin VARCHAR(2048) NULL,
  package_sha256 CHAR(64) NOT NULL,
  applied_package JSON NOT NULL,
  installed_by CHAR(36) NULL,
  installed_at DATETIME(6) NOT NULL,
  PRIMARY KEY (id),
  KEY config_package_versions_package_index (package_id, installed_at),
  CONSTRAINT config_package_versions_install_fk FOREIGN KEY (install_id) REFERENCES config_package_installs (id) ON DELETE CASCADE,
  CONSTRAINT config_package_versions_installed_by_fk FOREIGN KEY (installed_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE config_package_version_items (
  version_id CHAR(36) NOT NULL,
  item_id VARCHAR(255) NOT NULL,
  section VARCHAR(64) NOT NULL,
  kind VARCHAR(64) NOT NULL,
  item_key VARCHAR(191) NOT NULL,
  package_hash CHAR(64) NOT NULL,
  resolution VARCHAR(20) NOT NULL,
  PRIMARY KEY (version_id, item_id),
  CONSTRAINT config_package_version_items_version_fk FOREIGN KEY (version_id) REFERENCES config_package_versions (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
