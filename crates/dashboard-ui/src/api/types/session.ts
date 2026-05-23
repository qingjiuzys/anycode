import type { ArtifactRecord, GateRecord } from "./artifacts";
import type { ProjectEvent } from "./project";

export interface SessionWithProject {
  id: string;
  project_id: string;
  project_name: string;
  kind: string;
  task_id: string | null;
  title: string;
  status: string;
  trusted_status: string;
  agent_type: string;
  model: string;
  started_at: string;
  ended_at: string | null;
}

export interface SessionDetail {
  id: string;
  project_id: string;
  project_name: string;
  kind: string;
  task_id: string | null;
  title: string;
  prompt_preview: string;
  status: string;
  trusted_status: string;
  agent_type: string;
  model: string;
  started_at: string;
  ended_at: string | null;
  summary: string;
  metadata_json: string;
}

export interface SessionSummary {
  id: string;
  kind: string;
  task_id: string | null;
  title: string;
  status: string;
  trusted_status: string;
  agent_type: string;
  model: string;
  started_at: string;
  ended_at: string | null;
}

export interface SessionSummaryFailure {
  event_id: string;
  title: string;
  event_type: string;
  occurred_at: string;
  body: string;
}

export interface ToolCallSummary {
  event_id: string;
  title: string;
  body: string;
  occurred_at: string;
  event_type: string;
  tool_name?: string | null;
}

export interface TracePhaseSummary {
  phase: string;
  count: number;
  severity: string;
}

export interface SessionReplaySummary {
  session_id: string;
  project_id: string;
  project_name: string;
  title: string;
  status: string;
  trusted_status: string;
  kind: string;
  failed_gates: GateRecord[];
  last_error?: SessionSummaryFailure | null;
  artifacts: ArtifactRecord[];
  recent_events: ProjectEvent[];
  report_artifacts: ArtifactRecord[];
  generated_at: string;
  attempt_count?: number;
  active_agent?: string | null;
  tool_calls_recent?: ToolCallSummary[];
  trace_phases?: TracePhaseSummary[];
  llm_calls_count?: number;
  tool_calls_count?: number;
  budget_events_count?: number;
  budget_status?: string;
}
