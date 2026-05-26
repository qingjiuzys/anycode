import type { LabelCount } from "./core";
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
  block_reason?: string | null;
  block_kind?: string | null;
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
  block_reason?: string | null;
  block_kind?: string | null;
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
  block_reason?: string | null;
  block_kind?: string | null;
}

export interface SessionFacetsResponse {
  status: LabelCount[];
  trusted_status: LabelCount[];
  kind: LabelCount[];
  pending_approval_total: number;
}

export interface TranscriptBlock {
  id: string;
  block_type: string;
  at: string;
  title: string;
  body: string;
  meta?: Record<string, unknown>;
  collapsible?: boolean;
  default_collapsed?: boolean;
  event_id?: string | null;
}

export interface SessionTranscriptResponse {
  schema_version: number;
  session_id: string;
  blocks: TranscriptBlock[];
  lifecycle: TranscriptBlock[];
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

export interface SessionTraceEvent {
  event_type: string;
  severity: string;
  title: string;
  body: string;
  payload: Record<string, unknown>;
  occurred_at: string;
}

export interface SessionTraceResponse {
  schema_version: number;
  session_id: string;
  source?: string;
  events: SessionTraceEvent[];
}

export interface ExecutionLogLine {
  line_no: number;
  raw: string;
  event_type?: string | null;
  severity?: string | null;
  title?: string | null;
  body?: string | null;
}

export interface ExecutionLogResponse {
  session_id: string;
  task_id?: string | null;
  log_path?: string | null;
  offset: number;
  next_offset: number;
  has_more: boolean;
  lines: ExecutionLogLine[];
  source?: string;
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
