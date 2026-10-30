CREATE TABLE communication_opt_outs (
  id CHAR(36) NOT NULL,
  address VARCHAR(320) NOT NULL,
  scope VARCHAR(20) NOT NULL,
  reason VARCHAR(500) NULL,
  source VARCHAR(64) NOT NULL,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY communication_opt_outs_address_unique (address),
  CONSTRAINT communication_opt_outs_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
