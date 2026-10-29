CREATE TABLE report_schedule_recipients (
  schedule_id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  PRIMARY KEY (schedule_id, user_id),
  KEY report_schedule_recipients_user_fk_index (user_id),
  CONSTRAINT report_schedule_recipients_schedule_fk FOREIGN KEY (schedule_id) REFERENCES report_schedules (id) ON DELETE CASCADE,
  CONSTRAINT report_schedule_recipients_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
