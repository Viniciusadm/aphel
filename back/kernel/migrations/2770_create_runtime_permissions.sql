CREATE TABLE runtime_permissions (
  permission VARCHAR(100) NOT NULL,
  owner VARCHAR(64) NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  PRIMARY KEY (permission),
  KEY runtime_permissions_owner_index (owner)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
