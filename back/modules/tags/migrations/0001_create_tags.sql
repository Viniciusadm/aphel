CREATE TABLE tags (
  id CHAR(36) NOT NULL,
  name VARCHAR(60) NOT NULL,
  color VARCHAR(7) NOT NULL,
  record_types JSON NOT NULL DEFAULT (JSON_ARRAY()),
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY tags_name_unique (name),
  KEY tags_created_by_index (created_by),
  CONSTRAINT tags_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE taggings (
  tag_id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (tag_id, record_type, record_id),
  KEY taggings_record_index (record_type, record_id),
  KEY taggings_created_by_index (created_by),
  CONSTRAINT taggings_tag_fk FOREIGN KEY (tag_id) REFERENCES tags (id) ON DELETE CASCADE,
  CONSTRAINT taggings_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
