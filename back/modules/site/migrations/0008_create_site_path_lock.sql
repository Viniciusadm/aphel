CREATE TABLE site_path_lock (
  id TINYINT UNSIGNED NOT NULL,
  PRIMARY KEY (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT INTO site_path_lock (id) VALUES (1);
