CREATE TABLE field_permissions (
  field_id CHAR(36) NOT NULL,
  role_id CHAR(36) NOT NULL,
  record_type VARCHAR(64) NOT NULL,
  access VARCHAR(16) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (field_id, role_id),
  KEY field_permissions_lookup_index (record_type, role_id),
  CONSTRAINT field_permissions_field_fk FOREIGN KEY (field_id) REFERENCES field_definitions (id) ON DELETE CASCADE,
  CONSTRAINT field_permissions_role_fk FOREIGN KEY (role_id) REFERENCES roles (id) ON DELETE CASCADE,
  CONSTRAINT field_permissions_access_check CHECK (access IN ('hidden', 'read_only'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
