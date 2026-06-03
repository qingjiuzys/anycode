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
