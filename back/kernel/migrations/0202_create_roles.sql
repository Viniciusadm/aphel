CREATE TABLE roles (
  id CHAR(36) NOT NULL,
  `key` VARCHAR(64) NOT NULL,
  name VARCHAR(140) NOT NULL,
  is_system BOOLEAN NOT NULL DEFAULT FALSE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY roles_key_unique (`key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE role_permissions (
  role_id CHAR(36) NOT NULL,
  permission VARCHAR(100) NOT NULL,
  scope VARCHAR(20) NOT NULL DEFAULT 'organization',
  PRIMARY KEY (role_id, permission),
  CONSTRAINT role_permissions_role_fk FOREIGN KEY (role_id) REFERENCES roles (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

ALTER TABLE users
  ADD COLUMN role_id CHAR(36) NOT NULL AFTER status,
  ADD COLUMN invited_by CHAR(36) NULL AFTER role_id,
  ADD COLUMN joined_at DATETIME(6) NULL AFTER invited_by,
  ADD KEY users_role_index (role_id),
  ADD CONSTRAINT users_role_fk FOREIGN KEY (role_id) REFERENCES roles (id),
  ADD CONSTRAINT users_invited_by_fk FOREIGN KEY (invited_by) REFERENCES users (id) ON DELETE SET NULL;
