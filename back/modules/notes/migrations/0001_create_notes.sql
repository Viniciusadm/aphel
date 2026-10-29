CREATE TABLE notes (
  id CHAR(36) NOT NULL,
  author_id CHAR(36) NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  body MEDIUMTEXT NOT NULL,
  visibility VARCHAR(16) NOT NULL DEFAULT 'team',
  pinned BOOLEAN NOT NULL DEFAULT FALSE,
  deleted_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY notes_record_index (record_type, record_id, pinned, created_at),
  KEY notes_author_fk_index (author_id),
  CONSTRAINT notes_author_fk FOREIGN KEY (author_id) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
