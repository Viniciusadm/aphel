CREATE TABLE tenants (
  id CHAR(36) NOT NULL,
  slug VARCHAR(63) NOT NULL,
  name VARCHAR(140) NOT NULL,
  database_name VARCHAR(64) NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'provisioning',
  migrated_at DATETIME(6) NULL,
  last_error TEXT NULL,
  plugin_management_enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY tenants_slug_unique (slug),
  UNIQUE KEY tenants_database_name_unique (database_name),
  CONSTRAINT tenants_status_check CHECK (status IN ('provisioning', 'active', 'suspended', 'deleting'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE tenant_hosts (
  host VARCHAR(253) NOT NULL,
  tenant_id CHAR(36) NOT NULL,
  is_primary BOOLEAN NOT NULL DEFAULT FALSE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (host),
  KEY tenant_hosts_tenant_index (tenant_id),
  CONSTRAINT tenant_hosts_tenant_fk FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE operators (
  id CHAR(36) NOT NULL,
  email VARCHAR(255) NOT NULL,
  password_hash VARCHAR(255) NOT NULL,
  display_name VARCHAR(140) NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'active',
  invite_token_hash CHAR(64) NULL,
  invite_expires_at DATETIME(6) NULL,
  failed_logins INT UNSIGNED NOT NULL DEFAULT 0,
  locked_until DATETIME(6) NULL,
  last_login_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY operators_email_unique (email),
  UNIQUE KEY operators_invite_token_unique (invite_token_hash),
  CONSTRAINT operators_status_check CHECK (status IN ('invited', 'active', 'deactivated'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE operator_sessions (
  id CHAR(36) NOT NULL,
  operator_id CHAR(36) NOT NULL,
  token_hash CHAR(64) NOT NULL,
  ip VARCHAR(45) NULL,
  user_agent VARCHAR(255) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  last_seen_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  expires_at DATETIME(6) NOT NULL,
  revoked_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY operator_sessions_token_hash_unique (token_hash),
  KEY operator_sessions_operator_index (operator_id),
  CONSTRAINT operator_sessions_operator_fk FOREIGN KEY (operator_id) REFERENCES operators (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE operator_audit (
  id CHAR(36) NOT NULL,
  operator_id CHAR(36) NULL,
  action VARCHAR(100) NOT NULL,
  tenant_id CHAR(36) NULL,
  details JSON NOT NULL,
  ip VARCHAR(45) NULL,
  occurred_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY operator_audit_operator_index (operator_id, occurred_at),
  KEY operator_audit_tenant_index (tenant_id, occurred_at),
  KEY operator_audit_occurred_index (occurred_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
