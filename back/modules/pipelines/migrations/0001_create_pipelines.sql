CREATE TABLE pipelines (
  id CHAR(36) NOT NULL,
  name VARCHAR(140) NOT NULL,
  is_default BOOLEAN NOT NULL DEFAULT FALSE,
  position INT NOT NULL DEFAULT 0,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY pipelines_position_index (position)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE pipeline_stages (
  id CHAR(36) NOT NULL,
  pipeline_id CHAR(36) NOT NULL,
  name VARCHAR(140) NOT NULL,
  position INT NOT NULL DEFAULT 0,
  probability TINYINT UNSIGNED NOT NULL DEFAULT 0,
  rotting_days SMALLINT UNSIGNED NULL,
  kind VARCHAR(16) NOT NULL DEFAULT 'open',
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  entry_required_fields JSON NULL,
  exit_required_fields JSON NULL,
  PRIMARY KEY (id),
  KEY pipeline_stages_pipeline_index (pipeline_id, position),
  CONSTRAINT pipeline_stages_pipeline_fk FOREIGN KEY (pipeline_id) REFERENCES pipelines (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE pipeline_teams (
  pipeline_id CHAR(36) NOT NULL,
  team_id CHAR(36) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (pipeline_id, team_id),
  KEY pipeline_teams_team_fk_index (team_id),
  CONSTRAINT pipeline_teams_pipeline_fk FOREIGN KEY (pipeline_id) REFERENCES pipelines (id) ON DELETE CASCADE,
  CONSTRAINT pipeline_teams_team_fk FOREIGN KEY (team_id) REFERENCES teams (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE lost_reasons (
  id CHAR(36) NOT NULL,
  name VARCHAR(140) NOT NULL,
  position INT NOT NULL DEFAULT 0,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY lost_reasons_position_index (position)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE pipeline_stage_guidance (
  stage_id CHAR(36) NOT NULL,
  guidance TEXT NOT NULL,
  updated_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (stage_id),
  KEY pipeline_stage_guidance_updated_by_fk_index (updated_by),
  CONSTRAINT pipeline_stage_guidance_stage_fk FOREIGN KEY (stage_id) REFERENCES pipeline_stages (id) ON DELETE CASCADE,
  CONSTRAINT pipeline_stage_guidance_updated_by_fk FOREIGN KEY (updated_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE pipeline_stage_checklist_items (
  id CHAR(36) NOT NULL,
  stage_id CHAR(36) NOT NULL,
  `key` VARCHAR(64) NOT NULL,
  label VARCHAR(200) NOT NULL,
  is_required BOOLEAN NOT NULL DEFAULT FALSE,
  position INT NOT NULL DEFAULT 0,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY pipeline_stage_checklist_items_key_unique (stage_id, `key`),
  KEY pipeline_stage_checklist_items_stage_fk_index (stage_id, position),
  CONSTRAINT pipeline_stage_checklist_items_stage_fk FOREIGN KEY (stage_id) REFERENCES pipeline_stages (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
