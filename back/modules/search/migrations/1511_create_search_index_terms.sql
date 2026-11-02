SET SESSION innodb_ft_enable_stopword = OFF;

CREATE TABLE search_index_terms (
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  field_key VARCHAR(64) NOT NULL,
  position INT NOT NULL,
  weight TINYINT UNSIGNED NOT NULL,
  is_title_candidate BOOLEAN NOT NULL DEFAULT FALSE,
  value VARCHAR(1000) NOT NULL,
  content TEXT NOT NULL,
  PRIMARY KEY (record_type, record_id, field_key),
  KEY search_index_terms_field_index (record_type, field_key),
  FULLTEXT KEY search_index_terms_content_fulltext (content) WITH PARSER ngram,
  CONSTRAINT search_index_terms_record_fk FOREIGN KEY (record_type, record_id)
    REFERENCES search_index (record_type, record_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

SET SESSION innodb_ft_enable_stopword = ON;
