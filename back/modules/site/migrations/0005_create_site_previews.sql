CREATE TABLE site_previews (
  token_hash CHAR(64) NOT NULL,
  theme VARCHAR(64) NULL,
  expires_at DATETIME(6) NOT NULL,
  PRIMARY KEY (token_hash),
  KEY site_previews_expires_index (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
