CREATE TABLE record_relations (
  relationship_id CHAR(36) NOT NULL,
  from_id CHAR(36) NOT NULL,
  to_id CHAR(36) NOT NULL,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (relationship_id, from_id, to_id),
  KEY record_relations_to_index (relationship_id, to_id),
  KEY record_relations_from_record_index (from_id),
  KEY record_relations_to_record_index (to_id),
  KEY record_relations_created_by_fk_index (created_by),
  CONSTRAINT record_relations_relationship_fk FOREIGN KEY (relationship_id) REFERENCES relationship_definitions (id) ON DELETE CASCADE,
  CONSTRAINT record_relations_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
