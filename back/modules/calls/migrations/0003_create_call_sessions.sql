CREATE TABLE call_sessions (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  view_id CHAR(36) NULL,
  name VARCHAR(200) NOT NULL,
  script TEXT NULL,
  status VARCHAR(16) NOT NULL,
  position INT UNSIGNED NOT NULL DEFAULT 0,
  total INT UNSIGNED NOT NULL DEFAULT 0,
  completed_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY call_sessions_user_index (user_id, status, created_at),
  KEY call_sessions_view_fk_index (view_id),
  CONSTRAINT call_sessions_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT call_sessions_view_fk FOREIGN KEY (view_id) REFERENCES saved_views (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE call_session_items (
  session_id CHAR(36) NOT NULL,
  position INT UNSIGNED NOT NULL,
  contact_id CHAR(36) NOT NULL,
  status VARCHAR(16) NOT NULL,
  activity_id CHAR(36) NULL,
  outcome_key VARCHAR(64) NULL,
  handled_at DATETIME(6) NULL,
  PRIMARY KEY (session_id, position),
  KEY call_session_items_contact_index (contact_id),
  KEY call_session_items_activity_index (activity_id),
  CONSTRAINT call_session_items_session_fk FOREIGN KEY (session_id) REFERENCES call_sessions (id) ON DELETE CASCADE,
  CONSTRAINT call_session_items_activity_fk FOREIGN KEY (activity_id) REFERENCES activities (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
