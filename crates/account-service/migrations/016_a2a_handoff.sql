-- A2A agent presence + cloud handoff task metadata (no bundle bytes — stream relay only).

CREATE TABLE IF NOT EXISTS a2a_agent_presence (
  device_id VARCHAR(64) NOT NULL PRIMARY KEY,
  user_id VARCHAR(64) NOT NULL,
  organization_id VARCHAR(64) NOT NULL,
  instance_id VARCHAR(128) NOT NULL,
  agent_card_json JSON NOT NULL,
  last_heartbeat_at DATETIME(3) NOT NULL,
  KEY idx_a2a_presence_org (organization_id, last_heartbeat_at),
  KEY idx_a2a_presence_instance (instance_id)
);

CREATE TABLE IF NOT EXISTS a2a_handoff_tasks (
  id VARCHAR(64) NOT NULL PRIMARY KEY,
  organization_id VARCHAR(64) NOT NULL,
  kind VARCHAR(16) NOT NULL,
  state VARCHAR(32) NOT NULL,
  sender_user_id VARCHAR(64) NOT NULL,
  sender_device_id VARCHAR(64) NOT NULL,
  sender_instance_id VARCHAR(128) NOT NULL,
  recipient_user_id VARCHAR(64) NOT NULL,
  recipient_device_id VARCHAR(64) NOT NULL,
  recipient_instance_id VARCHAR(128) NOT NULL,
  project_id VARCHAR(128) NULL,
  project_name VARCHAR(256) NULL,
  session_id VARCHAR(128) NULL,
  session_title VARCHAR(256) NULL,
  target_project_id VARCHAR(128) NULL,
  stream_token_hash VARCHAR(128) NULL,
  progress_pct TINYINT UNSIGNED NOT NULL DEFAULT 0,
  error_message VARCHAR(512) NULL,
  created_at DATETIME(3) NOT NULL,
  updated_at DATETIME(3) NOT NULL,
  expires_at DATETIME(3) NOT NULL,
  KEY idx_a2a_handoff_org_state (organization_id, state),
  KEY idx_a2a_handoff_recipient (recipient_device_id, state),
  KEY idx_a2a_handoff_sender (sender_device_id, state)
);
