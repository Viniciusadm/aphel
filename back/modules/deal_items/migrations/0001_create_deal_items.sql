CREATE TABLE deal_items (
  id CHAR(36) NOT NULL,
  deal_id CHAR(36) NOT NULL,
  product_id CHAR(36) NULL,
  name VARCHAR(255) NOT NULL,
  unit VARCHAR(32) NULL,
  quantity DECIMAL(18,4) NOT NULL,
  unit_price DECIMAL(18,2) NOT NULL,
  discount_type VARCHAR(16) NOT NULL DEFAULT 'none',
  discount_value DECIMAL(18,2) NOT NULL DEFAULT 0,
  recurrence VARCHAR(16) NOT NULL DEFAULT 'none',
  term_months SMALLINT UNSIGNED NULL,
  total DECIMAL(18,2) NOT NULL,
  amount_base DECIMAL(18,2) NULL,
  exchange_rate DECIMAL(24,10) NULL,
  position INT NOT NULL DEFAULT 0,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY deal_items_deal_index (deal_id, position),
  KEY deal_items_product_fk_index (product_id),
  KEY deal_items_created_by_fk_index (created_by),
  CONSTRAINT deal_items_deal_fk FOREIGN KEY (deal_id) REFERENCES deals (id) ON DELETE CASCADE,
  CONSTRAINT deal_items_product_fk FOREIGN KEY (product_id) REFERENCES products (id) ON DELETE SET NULL,
  CONSTRAINT deal_items_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE deal_item_settings (
  id TINYINT UNSIGNED NOT NULL,
  amount_mode VARCHAR(16) NOT NULL DEFAULT 'manual',
  updated_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  CONSTRAINT deal_item_settings_single_row CHECK (id = 1),
  KEY deal_item_settings_updated_by_fk_index (updated_by),
  CONSTRAINT deal_item_settings_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
