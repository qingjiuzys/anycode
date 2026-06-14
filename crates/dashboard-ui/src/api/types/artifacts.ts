export interface GateRecord {
  id: string;
  name: string;
  status: string;
  required: boolean;
  output_excerpt: string;
  session_id: string | null;
}

export interface ArtifactRecord {
  id: string;
  path: string;
  kind: string;
  title: string;
  trust_level: string;
  verified_by_gate_id: string | null;
  session_id?: string | null;
  project_id?: string | null;
  project_name?: string | null;
  verified_by_gate_name?: string | null;
  session_trusted_status?: string | null;
  updated_at?: string | null;
}

export interface ArtifactVersionRecord {
  id: string;
  artifact_id: string;
  hash: string;
  size_bytes: number;
  indexed_at: string;
  summary: string;
}

export interface ArtifactLinkRecord {
  id: string;
  artifact_id: string;
  link_type: string;
  target_id?: string | null;
  target_url?: string | null;
  created_at: string;
}

export interface ArtifactDetail {
  artifact: ArtifactRecord;
  versions: ArtifactVersionRecord[];
  links: ArtifactLinkRecord[];
  report_markdown?: string | null;
}

export type AssetKind =
  | "deliverable"
  | "media"
  | "report"
  | "workflow"
  | "skill"
  | "all";

export type AssetSourceType =
  | "agent_created"
  | "user_added"
  | "workspace_scan"
  | "report_archive"
  | "skill_scan"
  | "workflow_scan";

export type AssetReuseState = "candidate" | "reusable" | "archived";

export interface AssetItem {
  id: string;
  title: string;
  subtitle: string;
  asset_kind: AssetKind | string;
  backend_type: "artifact" | "skill" | string;
  backend_id: string;
  project_id?: string | null;
  project_name?: string | null;
  session_id?: string | null;
  trust_level: string;
  source_type: AssetSourceType | string;
  reuse_state: AssetReuseState | string;
  path?: string | null;
  category?: string | null;
  note?: string | null;
  tags: string[];
  updated_at?: string | null;
  verified_by_gate_name?: string | null;
  session_trusted_status?: string | null;
  skill_enabled?: boolean | null;
}

export interface AssetDetail {
  asset: AssetItem;
  artifact?: ArtifactDetail | null;
  skill?: import("./governance").SkillDetailRecord | null;
  promotion_draft_path?: string | null;
}

export interface AssetActionRequest {
  note?: string;
  tags?: string[];
}

export interface AssetActionResult {
  ok: boolean;
  asset: AssetItem;
  draft_path?: string | null;
}

export interface IndexAssetsResult {
  indexed: number;
  missing: number;
  skipped: number;
  total: number;
  job_id: string;
}

export interface GatePreset {
  id: string;
  name: string;
  command: string;
}

export interface GateExecuteResult {
  name: string;
  command: string;
  status: string;
  output_excerpt: string;
  elapsed_ms: number;
}
