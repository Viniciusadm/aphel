CREATE TABLE price_lists (
  id CHAR(36) NOT NULL,
  `key` VARCHAR(64) NOT NULL,
  name VARCHAR(140) NOT NULL,
  description TEXT NULL,
  currency CHAR(3) NOT NULL,
  applies_to VARCHAR(12) NOT NULL DEFAULT 'all',
  segment_id CHAR(36) NULL,
  valid_from DATE NULL,
  valid_to DATE NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  is_default BOOLEAN NOT NULL DEFAULT FALSE,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  default_marker CHAR(3) GENERATED ALWAYS AS (CASE WHEN is_default THEN currency END) STORED,
  PRIMARY KEY (id),
  UNIQUE KEY price_lists_key_unique (`key`),
  UNIQUE KEY price_lists_default_unique (default_marker),
  KEY price_lists_lookup_index (currency, is_active, applies_to),
  KEY price_lists_segment_fk_index (segment_id),
  KEY price_lists_created_by_fk_index (created_by),
  CONSTRAINT price_lists_segment_fk FOREIGN KEY (segment_id) REFERENCES segments (id) ON DELETE SET NULL,
  CONSTRAINT price_lists_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL,
  CONSTRAINT price_lists_applies_to_check CHECK (applies_to IN ('all', 'segment', 'companies'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE price_list_companies (
  price_list_id CHAR(36) NOT NULL,
  company_id CHAR(36) NOT NULL,
  PRIMARY KEY (price_list_id, company_id),
  KEY price_list_companies_company_fk_index (company_id),
  CONSTRAINT price_list_companies_price_list_fk FOREIGN KEY (price_list_id) REFERENCES price_lists (id) ON DELETE CASCADE,
  CONSTRAINT price_list_companies_company_fk FOREIGN KEY (company_id) REFERENCES companies (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE price_list_items (
  id CHAR(36) NOT NULL,
  price_list_id CHAR(36) NOT NULL,
  product_id CHAR(36) NOT NULL,
  min_quantity DECIMAL(18,4) NOT NULL DEFAULT 1,
  unit_price DECIMAL(18,2) NOT NULL,
  position INT NOT NULL DEFAULT 0,
  PRIMARY KEY (id),
  UNIQUE KEY price_list_items_tier_unique (price_list_id, product_id, min_quantity),
  KEY price_list_items_product_fk_index (product_id),
  CONSTRAINT price_list_items_price_list_fk FOREIGN KEY (price_list_id) REFERENCES price_lists (id) ON DELETE CASCADE,
  CONSTRAINT price_list_items_product_fk FOREIGN KEY (product_id) REFERENCES products (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE product_bundles (
  product_id CHAR(36) NOT NULL,
  pricing VARCHAR(8) NOT NULL,
  updated_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (product_id),
  KEY product_bundles_updated_by_fk_index (updated_by),
  CONSTRAINT product_bundles_product_fk FOREIGN KEY (product_id) REFERENCES products (id) ON DELETE CASCADE,
  CONSTRAINT product_bundles_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL,
  CONSTRAINT product_bundles_pricing_check CHECK (pricing IN ('fixed', 'sum'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE product_bundle_components (
  bundle_product_id CHAR(36) NOT NULL,
  component_product_id CHAR(36) NOT NULL,
  quantity DECIMAL(18,4) NOT NULL,
  position INT NOT NULL DEFAULT 0,
  PRIMARY KEY (bundle_product_id, component_product_id),
  KEY product_bundle_components_component_fk_index (component_product_id),
  CONSTRAINT product_bundle_components_bundle_fk FOREIGN KEY (bundle_product_id) REFERENCES product_bundles (product_id) ON DELETE CASCADE,
  CONSTRAINT product_bundle_components_component_fk FOREIGN KEY (component_product_id) REFERENCES products (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE role_discount_limits (
  role_id CHAR(36) NOT NULL,
  max_discount_percent DECIMAL(5,2) NOT NULL,
  updated_by CHAR(36) NULL,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (role_id),
  KEY role_discount_limits_updated_by_fk_index (updated_by),
  CONSTRAINT role_discount_limits_role_fk FOREIGN KEY (role_id) REFERENCES roles (id) ON DELETE CASCADE,
  CONSTRAINT role_discount_limits_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL,
  CONSTRAINT role_discount_limits_percent_check CHECK (max_discount_percent >= 0 AND max_discount_percent <= 100)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

