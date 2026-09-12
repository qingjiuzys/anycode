-- Desktop pairing devices. Cloud SSO login alone is not device consent.
-- Only the token hash is stored; the opaque token is returned once.
CREATE TABLE IF NOT EXISTS harness_devices (
    id TEXT PRIMARY KEY,
    accounts_sub TEXT NOT NULL,
    label TEXT NOT NULL,
    token_hash TEXT,
    challenge_hash TEXT,
    status TEXT NOT NULL CHECK (status IN ('pending', 'active', 'revoked')),
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    revoked_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_harness_devices_sub_status
    ON harness_devices(accounts_sub, status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_harness_devices_token_hash
    ON harness_devices(token_hash) WHERE token_hash IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_harness_devices_challenge_hash
    ON harness_devices(challenge_hash) WHERE challenge_hash IS NOT NULL;
