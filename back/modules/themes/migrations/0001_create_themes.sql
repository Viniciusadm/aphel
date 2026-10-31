CREATE TABLE themes_installed (
  theme_key VARCHAR(64) NOT NULL,
  name VARCHAR(140) NOT NULL,
  version VARCHAR(64) NOT NULL,
  author VARCHAR(140) NOT NULL,
  description TEXT NULL,
  license VARCHAR(64) NOT NULL,
  min_aphel_version VARCHAR(16) NULL,
  tokens JSON NOT NULL,
  source VARCHAR(16) NOT NULL,
  checksum CHAR(64) NOT NULL,
  installed_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (theme_key),
  KEY themes_installed_user_fk_index (installed_by),
  CONSTRAINT themes_installed_user_fk FOREIGN KEY (installed_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE themes_fonts (
  id BIGINT NOT NULL AUTO_INCREMENT,
  theme_key VARCHAR(64) NOT NULL,
  family VARCHAR(64) NOT NULL,
  weight SMALLINT NOT NULL,
  style VARCHAR(8) NOT NULL,
  sha256 CHAR(64) NOT NULL,
  size INT NOT NULL,
  data MEDIUMBLOB NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY themes_fonts_face_unique (theme_key, family, weight, style),
  KEY themes_fonts_sha_index (sha256),
  CONSTRAINT themes_fonts_theme_fk FOREIGN KEY (theme_key) REFERENCES themes_installed (theme_key) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE themes_state (
  id TINYINT NOT NULL,
  active_key VARCHAR(64) NULL,
  version BIGINT NOT NULL DEFAULT 0,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY themes_state_active_index (active_key),
  CONSTRAINT themes_state_single_row CHECK (id = 1),
  CONSTRAINT themes_state_active_fk FOREIGN KEY (active_key) REFERENCES themes_installed (theme_key) ON DELETE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT INTO themes_state (id, active_key, version) VALUES (1, NULL, 0);
