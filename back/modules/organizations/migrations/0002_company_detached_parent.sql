ALTER TABLE organizations
  ADD COLUMN detached_from_company_id CHAR(36) NULL AFTER parent_company_id,
  ADD INDEX organizations_detached_from_idx (detached_from_company_id);
