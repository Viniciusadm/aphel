CREATE TABLE publication_subjects (
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  active_revision_id CHAR(36) NULL,
  generation BIGINT UNSIGNED NOT NULL DEFAULT 0,
  first_published_at DATETIME(6) NULL,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (record_type, record_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE publication_revisions (
  id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  number INT UNSIGNED NOT NULL,
  format INT UNSIGNED NOT NULL,
  payload JSON NOT NULL,
  payload_hash CHAR(64) NOT NULL,
  draft_version DATETIME(6) NOT NULL,
  note VARCHAR(280) NULL,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY publication_revisions_number_unique (record_type, record_id, number),
  KEY publication_revisions_retention_index (record_type, record_id, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE publication_revision_files (
  revision_id CHAR(36) NOT NULL,
  file_id CHAR(36) NOT NULL,
  PRIMARY KEY (revision_id, file_id),
  KEY publication_revision_files_file_index (file_id),
  CONSTRAINT publication_revision_files_revision_fk FOREIGN KEY (revision_id) REFERENCES publication_revisions (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE publication_requests (
  id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  action VARCHAR(12) NOT NULL,
  revision_id CHAR(36) NULL,
  generation BIGINT UNSIGNED NOT NULL,
  expected_active_revision_id CHAR(36) NULL,
  status VARCHAR(20) NOT NULL,
  run_at DATETIME(6) NULL,
  requested_by CHAR(36) NULL,
  idempotency_key VARCHAR(80) NOT NULL,
  input_hash CHAR(64) NOT NULL,
  attempts INT UNSIGNED NOT NULL DEFAULT 0,
  reason_code VARCHAR(64) NULL,
  created_at DATETIME(6) NOT NULL,
  completed_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY publication_requests_idempotency_unique (requested_by, idempotency_key),
  KEY publication_requests_subject_index (record_type, record_id, status),
  KEY publication_requests_revision_fk_index (revision_id),
  CONSTRAINT publication_requests_revision_fk FOREIGN KEY (revision_id) REFERENCES publication_revisions (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
