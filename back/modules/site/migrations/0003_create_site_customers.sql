CREATE TABLE site_customers (
  id CHAR(36) NOT NULL,
  email VARCHAR(255) NOT NULL,
  name VARCHAR(200) NOT NULL DEFAULT '',
  password_hash VARCHAR(255) NULL,
  email_verified_at DATETIME(6) NULL,
  last_login_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  deleted_at DATETIME(6) NULL,
  live_email VARCHAR(255) GENERATED ALWAYS AS (IF(deleted_at IS NULL, email, NULL)) STORED,
  PRIMARY KEY (id),
  UNIQUE KEY site_customers_live_email_unique (live_email),
  KEY site_customers_created_at_index (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE site_customer_sessions (
  id CHAR(36) NOT NULL,
  customer_id CHAR(36) NOT NULL,
  token_hash CHAR(64) NOT NULL,
  ip VARCHAR(64) NULL,
  user_agent VARCHAR(255) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  last_seen_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  expires_at DATETIME(6) NOT NULL,
  revoked_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY site_customer_sessions_token_unique (token_hash),
  KEY site_customer_sessions_customer_index (customer_id),
  CONSTRAINT site_customer_sessions_customer_fk FOREIGN KEY (customer_id) REFERENCES site_customers (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE site_customer_tokens (
  id CHAR(36) NOT NULL,
  customer_id CHAR(36) NOT NULL,
  purpose VARCHAR(20) NOT NULL,
  token_hash CHAR(64) NOT NULL,
  expires_at DATETIME(6) NOT NULL,
  used_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY site_customer_tokens_token_unique (token_hash),
  KEY site_customer_tokens_customer_index (customer_id, purpose),
  CONSTRAINT site_customer_tokens_customer_fk FOREIGN KEY (customer_id) REFERENCES site_customers (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
