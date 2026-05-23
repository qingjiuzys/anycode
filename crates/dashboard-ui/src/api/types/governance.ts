export interface AuditRecord {
  id: string;
  project_id?: string | null;
  session_id?: string | null;
  actor: string;
  action: string;
  risk: string;
  detail: Record<string, unknown>;
  created_at: string;
}

export interface AutomationPolicyRecord {
  id: string;
  project_id: string;
  name: string;
  enabled: boolean;
  policy_type: string;
  config: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface SkillRecord {
  id: string;
  name: string;
  description: string;
  source_path: string;
  projects_count: number;
  enabled?: boolean;
}

export interface SkillDetailRecord {
  id: string;
  name: string;
  description: string;
  source_path: string;
  permissions: Record<string, unknown>;
  projects_count: number;
  projects: SkillProjectLink[];
  recent_runs: SkillRunRecord[];
}

export interface SkillProjectLink {
  project_id: string;
  project_name: string;
  enabled: boolean;
}

export interface SkillRunRecord {
  id: string;
  skill_id: string;
  project_id?: string | null;
  session_id?: string | null;
  status: string;
  started_at: string;
  ended_at?: string | null;
}

export interface SecurityEventRecord {
  id: string;
  project_id: string;
  project_name: string;
  session_id?: string | null;
  event_type: string;
  severity: string;
  title: string;
  tool_name: string;
  reason?: string | null;
  occurred_at: string;
}

export interface SecurityActivitySummary {
  denied_total: number;
  pending_total: number;
  recent: SecurityEventRecord[];
  read_only: boolean;
  note: string;
}

export interface ToolGovernanceEntry {
  id: string;
  category: string;
  risk_tier: string;
  default_agents: string[];
  requires_approval: boolean;
  audit_level: string;
}

export interface ToolGovernanceResponse {
  summary: {
    total: number;
    high_risk: number;
    approval_gaps: number;
  };
  tools: ToolGovernanceEntry[];
}

export type ApprovalDecision = "allow_once" | "deny" | "allow_tool";

export interface PendingApprovalRecord {
  approval_id: string;
  session_id: string;
  tool: string;
  input_preview: string;
  created_at: string;
  status: string;
}

export interface PendingApprovalsResponse {
  pending: PendingApprovalRecord[];
  web_enabled: boolean;
  respond_allowed: boolean;
}

export interface PendingApprovalSessionCount {
  session_id: string;
  count: number;
}

export interface PendingApprovalSummary {
  pending_total: number;
  by_session: PendingApprovalSessionCount[];
}

export interface ApprovalSummaryResponse {
  summary: PendingApprovalSummary;
  web_enabled: boolean;
  respond_allowed: boolean;
}
