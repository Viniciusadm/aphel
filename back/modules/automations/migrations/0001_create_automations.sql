CREATE TABLE automations (
  id CHAR(36) NOT NULL,
  `key` VARCHAR(64) NOT NULL,
  name VARCHAR(140) NOT NULL,
  description VARCHAR(1000) NULL,
  record_type VARCHAR(64) NOT NULL,
  `trigger` JSON NOT NULL,
  definition JSON NOT NULL,
  status VARCHAR(16) NOT NULL DEFAULT 'draft',
  version INT NOT NULL DEFAULT 0,
  created_by CHAR(36) NULL,
  updated_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY automations_key_unique (`key`),
  KEY automations_status_index (status, record_type),
  KEY automations_created_by_fk_index (created_by),
  KEY automations_updated_by_fk_index (updated_by),
  CONSTRAINT automations_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL,
  CONSTRAINT automations_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE automation_versions (
  id CHAR(36) NOT NULL,
  automation_id CHAR(36) NOT NULL,
  version INT NOT NULL,
  name VARCHAR(140) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  trigger_type VARCHAR(64) NOT NULL,
  `trigger` JSON NOT NULL,
  definition JSON NOT NULL,
  published_by CHAR(36) NULL,
  published_at DATETIME(6) NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY automation_versions_number_unique (automation_id, version),
  KEY automation_versions_trigger_index (record_type, trigger_type),
  KEY automation_versions_trigger_type_index (trigger_type, automation_id),
  KEY automation_versions_published_by_fk_index (published_by),
  CONSTRAINT automation_versions_automation_fk FOREIGN KEY (automation_id) REFERENCES automations (id) ON DELETE CASCADE,
  CONSTRAINT automation_versions_published_by_fk FOREIGN KEY (published_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE automation_runs (
  id CHAR(36) NOT NULL,
  automation_id CHAR(36) NOT NULL,
  version INT NOT NULL,
  trigger_key VARCHAR(120) NOT NULL,
  trigger_event_id CHAR(36) NULL,
  trigger_type VARCHAR(64) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  status VARCHAR(16) NOT NULL,
  depth INT NOT NULL DEFAULT 0,
  chain JSON NOT NULL,
  reason VARCHAR(64) NULL,
  error VARCHAR(1000) NULL,
  attempts INT NOT NULL DEFAULT 0,
  started_at DATETIME(6) NOT NULL,
  finished_at DATETIME(6) NULL,
  resume_at DATETIME(6) NULL,
  waiting_step_id VARCHAR(64) NULL,
  trigger_context JSON NULL,
  PRIMARY KEY (id),
  UNIQUE KEY automation_runs_trigger_unique (automation_id, trigger_key),
  KEY automation_runs_automation_index (automation_id, started_at),
  KEY automation_runs_record_index (automation_id, record_id, started_at),
  KEY automation_runs_resume_index (status, resume_at),
  KEY automation_runs_target_index (record_type, record_id, status),
  KEY automation_runs_failures_index (status, started_at),
  CONSTRAINT automation_runs_automation_fk FOREIGN KEY (automation_id) REFERENCES automations (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE automation_run_steps (
  id CHAR(36) NOT NULL,
  run_id CHAR(36) NOT NULL,
  position INT NOT NULL,
  step_id VARCHAR(64) NOT NULL,
  kind VARCHAR(32) NOT NULL,
  action VARCHAR(64) NULL,
  input JSON NULL,
  condition_result BOOLEAN NULL,
  next_step_id VARCHAR(64) NULL,
  output JSON NULL,
  status VARCHAR(16) NOT NULL,
  error VARCHAR(1000) NULL,
  started_at DATETIME(6) NOT NULL,
  duration_ms INT NOT NULL DEFAULT 0,
  PRIMARY KEY (id),
  UNIQUE KEY automation_run_steps_step_unique (run_id, step_id),
  KEY automation_run_steps_run_index (run_id, position),
  CONSTRAINT automation_run_steps_run_fk FOREIGN KEY (run_id) REFERENCES automation_runs (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE automation_time_cursors (
  automation_id CHAR(36) NOT NULL,
  version INT NOT NULL,
  cursor_at DATETIME(6) NOT NULL,
  page_offset BIGINT NOT NULL DEFAULT 0,
  occurrence_at DATETIME(6) NULL,
  swept_at DATETIME(6) NOT NULL,
  PRIMARY KEY (automation_id),
  KEY automation_time_cursors_swept_index (swept_at),
  CONSTRAINT automation_time_cursors_automation_fk FOREIGN KEY (automation_id) REFERENCES automations (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE INDEX custom_records_updated_index ON custom_records (object_id, updated_at);
