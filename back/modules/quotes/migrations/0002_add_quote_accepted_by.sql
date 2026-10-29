ALTER TABLE quotes
  ADD COLUMN accepted_by CHAR(36) NULL AFTER accepted_user_agent;
