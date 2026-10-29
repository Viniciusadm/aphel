CREATE TABLE push_subscriptions (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  session_id CHAR(36) NOT NULL,
  endpoint VARCHAR(1024) NOT NULL,
  endpoint_hash CHAR(64) NOT NULL,
  p256dh VARCHAR(128) NOT NULL,
  auth VARCHAR(64) NOT NULL,
  user_agent VARCHAR(255) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  last_used_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY push_subscriptions_endpoint_unique (endpoint_hash),
  KEY push_subscriptions_user_index (user_id),
  KEY push_subscriptions_session_index (session_id),
  CONSTRAINT push_subscriptions_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT push_subscriptions_session_fk FOREIGN KEY (session_id) REFERENCES sessions (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
