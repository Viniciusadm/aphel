CREATE TABLE money_currencies (
  currency CHAR(3) NOT NULL,
  enabled_by CHAR(36) NULL,
  enabled_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (currency),
  KEY money_currencies_enabled_by_index (enabled_by),
  CONSTRAINT money_currencies_enabled_by_fk FOREIGN KEY (enabled_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE money_exchange_rates (
  id CHAR(36) NOT NULL,
  currency CHAR(3) NOT NULL,
  effective_date DATE NOT NULL,
  rate DECIMAL(24,10) NOT NULL,
  source VARCHAR(16) NOT NULL DEFAULT 'manual',
  provider VARCHAR(64) NULL,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY exchange_rates_date_unique (currency, effective_date),
  KEY exchange_rates_created_by_index (created_by),
  CONSTRAINT exchange_rates_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
