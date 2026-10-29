CREATE TABLE mailboxes (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  provider VARCHAR(64) NOT NULL,
  address VARCHAR(320) NOT NULL,
  display_name VARCHAR(200) NULL,
  status VARCHAR(16) NOT NULL,
  status_reason VARCHAR(500) NULL,
  settings JSON NOT NULL,
  sync_cursor JSON NULL,
  history_days INT NOT NULL DEFAULT 90,
  default_visibility VARCHAR(16) NOT NULL DEFAULT 'metadata',
  sync_started_at DATETIME(6) NULL,
  last_synced_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY mailboxes_owner_address_unique (user_id, address),
  KEY mailboxes_status_index (status, last_synced_at),
  CONSTRAINT mailboxes_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE email_threads (
  id CHAR(36) NOT NULL,
  mailbox_id CHAR(36) NOT NULL,
  subject VARCHAR(1000) NOT NULL DEFAULT '',
  visibility VARCHAR(16) NULL,
  message_count INT NOT NULL DEFAULT 0,
  snippet VARCHAR(300) NOT NULL DEFAULT '',
  last_direction VARCHAR(16) NULL,
  first_message_at DATETIME(6) NULL,
  last_message_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY email_threads_mailbox_index (mailbox_id, last_message_at),
  CONSTRAINT email_threads_mailbox_fk FOREIGN KEY (mailbox_id) REFERENCES mailboxes (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE email_messages (
  id CHAR(36) NOT NULL,
  mailbox_id CHAR(36) NOT NULL,
  thread_id CHAR(36) NOT NULL,
  direction VARCHAR(16) NOT NULL,
  kind VARCHAR(16) NOT NULL DEFAULT 'message',
  status VARCHAR(16) NOT NULL,
  message_id_header VARCHAR(998) NOT NULL,
  message_id_hash CHAR(64) NOT NULL,
  in_reply_to VARCHAR(998) NULL,
  references_header TEXT NULL,
  report_for_message_id VARCHAR(998) NULL,
  provider_message_id VARCHAR(255) NULL,
  subject VARCHAR(1000) NOT NULL DEFAULT '',
  preview VARCHAR(300) NOT NULL DEFAULT '',
  body_file_id CHAR(36) NULL,
  has_html TINYINT(1) NOT NULL DEFAULT 0,
  has_attachments TINYINT(1) NOT NULL DEFAULT 0,
  tracking_enabled TINYINT(1) NOT NULL DEFAULT 0,
  sent_at DATETIME(6) NULL,
  scheduled_at DATETIME(6) NULL,
  opened_at DATETIME(6) NULL,
  open_count INT NOT NULL DEFAULT 0,
  clicked_at DATETIME(6) NULL,
  click_count INT NOT NULL DEFAULT 0,
  replied_at DATETIME(6) NULL,
  bounced_at DATETIME(6) NULL,
  failure_reason VARCHAR(500) NULL,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY email_messages_message_id_unique (mailbox_id, message_id_hash),
  KEY email_messages_thread_index (thread_id, sent_at),
  KEY email_messages_status_index (status, scheduled_at),
  KEY email_messages_body_file_fk_index (body_file_id),
  KEY email_messages_created_by_fk_index (created_by),
  CONSTRAINT email_messages_mailbox_fk FOREIGN KEY (mailbox_id) REFERENCES mailboxes (id) ON DELETE CASCADE,
  CONSTRAINT email_messages_thread_fk FOREIGN KEY (thread_id) REFERENCES email_threads (id) ON DELETE CASCADE,
  CONSTRAINT email_messages_body_file_fk FOREIGN KEY (body_file_id) REFERENCES files (id) ON DELETE SET NULL,
  CONSTRAINT email_messages_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE email_participants (
  id CHAR(36) NOT NULL,
  message_id CHAR(36) NOT NULL,
  role VARCHAR(16) NOT NULL,
  address VARCHAR(320) NOT NULL,
  name VARCHAR(200) NULL,
  contact_id CHAR(36) NULL,
  position INT NOT NULL DEFAULT 0,
  PRIMARY KEY (id),
  KEY email_participants_message_index (message_id, role, position),
  KEY email_participants_address_index (address),
  KEY email_participants_contact_index (contact_id),
  CONSTRAINT email_participants_message_fk FOREIGN KEY (message_id) REFERENCES email_messages (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE email_record_links (
  id CHAR(36) NOT NULL,
  thread_id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  record_id CHAR(36) NOT NULL,
  origin VARCHAR(16) NOT NULL,
  created_by CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY email_record_links_unique (thread_id, record_type, record_id),
  KEY email_record_links_record_index (record_type, record_id),
  KEY email_record_links_created_by_fk_index (created_by),
  CONSTRAINT email_record_links_thread_fk FOREIGN KEY (thread_id) REFERENCES email_threads (id) ON DELETE CASCADE,
  CONSTRAINT email_record_links_created_by_fk FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE email_attachments (
  id CHAR(36) NOT NULL,
  message_id CHAR(36) NOT NULL,
  file_id CHAR(36) NOT NULL,
  name VARCHAR(255) NOT NULL,
  mime_type VARCHAR(255) NOT NULL,
  size BIGINT UNSIGNED NOT NULL,
  content_id VARCHAR(255) NULL,
  inline TINYINT(1) NOT NULL DEFAULT 0,
  PRIMARY KEY (id),
  KEY email_attachments_message_index (message_id),
  KEY email_attachments_file_fk_index (file_id),
  CONSTRAINT email_attachments_message_fk FOREIGN KEY (message_id) REFERENCES email_messages (id) ON DELETE CASCADE,
  CONSTRAINT email_attachments_file_fk FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE mailbox_sync_failures (
  mailbox_id CHAR(36) NOT NULL,
  kind VARCHAR(32) NOT NULL,
  failures INT NOT NULL DEFAULT 0,
  retry_after DATETIME(6) NULL,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (mailbox_id),
  CONSTRAINT mailbox_sync_failures_mailbox_fk FOREIGN KEY (mailbox_id) REFERENCES mailboxes (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
