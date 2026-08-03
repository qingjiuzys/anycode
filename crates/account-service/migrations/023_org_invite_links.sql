-- Shareable team invite links (any signed-in solo owner can accept).

ALTER TABLE org_invites
  ADD COLUMN kind VARCHAR(16) NOT NULL DEFAULT 'email' AFTER organization_id;
