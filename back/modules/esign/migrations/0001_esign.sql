CREATE TABLE esign_requests (
  id CHAR(36) NOT NULL,
  deal_id CHAR(36) NOT NULL,
  file_id CHAR(36) NOT NULL,
  file_name VARCHAR(255) NOT NULL,
  provider VARCHAR(32) NOT NULL,
  environment VARCHAR(16) NOT NULL,
  provider_envelope_id VARCHAR(128) NOT NULL,
  provider_document_id VARCHAR(128) NULL,
  status VARCHAR(16) NOT NULL,
  message VARCHAR(1000) NULL,
  signed_file_id CHAR(36) NULL,
  created_by CHAR(36) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  completed_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  KEY esign_requests_deal_index (deal_id, created_at),
  KEY esign_requests_envelope_index (provider_envelope_id),
  KEY esign_requests_document_index (provider_document_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE esign_signers (
  id CHAR(36) NOT NULL,
  request_id CHAR(36) NOT NULL,
  position INT UNSIGNED NOT NULL,
  contact_id CHAR(36) NULL,
  name VARCHAR(255) NOT NULL,
  email VARCHAR(254) NOT NULL,
  provider_signer_id VARCHAR(128) NULL,
  status VARCHAR(16) NOT NULL,
  signed_at DATETIME(6) NULL,
  refused_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  KEY esign_signers_request_index (request_id, position),
  CONSTRAINT esign_signers_request_fk FOREIGN KEY (request_id) REFERENCES esign_requests (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE esign_webhook_receipts (
  id CHAR(36) NOT NULL,
  body_hash CHAR(64) NOT NULL,
  request_id CHAR(36) NULL,
  event_name VARCHAR(64) NOT NULL,
  received_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY esign_webhook_receipts_hash_unique (body_hash)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
