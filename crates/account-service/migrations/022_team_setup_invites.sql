-- Explicit team setup + email invites before org-scoped colleague discovery.

ALTER TABLE organizations
  ADD COLUMN team_setup_at DATETIME(3) NULL AFTER sso_status;

CREATE TABLE IF NOT EXISTS org_invites (
  id VARCHAR(64) NOT NULL PRIMARY KEY,
  organization_id VARCHAR(64) NOT NULL,
  email VARCHAR(255) NOT NULL,
  invited_by_user_id VARCHAR(64) NOT NULL,
  token_hash VARCHAR(128) NOT NULL,
  status VARCHAR(32) NOT NULL DEFAULT 'pending',
  expires_at DATETIME(3) NOT NULL,
  accepted_at DATETIME(3) NULL,
  created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_org_invites_org_email (organization_id, email),
  KEY idx_org_invites_org (organization_id, status),
  KEY idx_org_invites_email (email, status),
  CONSTRAINT fk_org_invites_org FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE
);
