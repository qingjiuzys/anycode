-- Explicit 818cloud SSO subject → local project membership.
-- SSO introspect success alone never authorizes a project.
CREATE TABLE IF NOT EXISTS harness_accounts_members (
    project_id TEXT NOT NULL,
    accounts_sub TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (project_id, accounts_sub),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_harness_accounts_members_sub
    ON harness_accounts_members(accounts_sub);
