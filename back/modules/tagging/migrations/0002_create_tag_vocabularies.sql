CREATE TABLE tag_vocabularies (
  id CHAR(36) NOT NULL,
  `key` VARCHAR(48) NOT NULL,
  name VARCHAR(80) NOT NULL,
  hierarchical BOOLEAN NOT NULL DEFAULT FALSE,
  public BOOLEAN NOT NULL DEFAULT FALSE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY tag_vocabularies_key_unique (`key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT INTO tag_vocabularies (id, `key`, name) VALUES ('00000000-0000-7000-8000-000000000001', 'general', 'Etiquetas');

ALTER TABLE tags
  ADD COLUMN vocabulary_id CHAR(36) NOT NULL DEFAULT '00000000-0000-7000-8000-000000000001' AFTER id,
  ADD COLUMN slug VARCHAR(80) NULL AFTER name,
  ADD COLUMN parent_id CHAR(36) NULL AFTER slug,
  DROP INDEX tags_name_unique,
  ADD UNIQUE KEY tags_vocabulary_name_unique (vocabulary_id, name),
  ADD UNIQUE KEY tags_vocabulary_slug_unique (vocabulary_id, slug),
  ADD KEY tags_parent_index (parent_id),
  ADD CONSTRAINT tags_vocabulary_fk FOREIGN KEY (vocabulary_id) REFERENCES tag_vocabularies (id),
  ADD CONSTRAINT tags_parent_fk FOREIGN KEY (parent_id) REFERENCES tags (id);

ALTER TABLE tags ALTER COLUMN vocabulary_id DROP DEFAULT;
