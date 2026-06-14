import { get, post } from "../http";
import type {
  AssetActionRequest,
  AssetActionResult,
  AssetDetail,
  AssetItem,
  ArtifactDetail,
  ArtifactRecord,
  IndexAssetsResult,
} from "../types/artifacts";

export type AssetListOpts = {
  projectId?: string;
  sessionId?: string;
  assetKind?: string;
  sourceType?: string;
  reuseState?: string;
  trustLevel?: string;
  unverifiedOnly?: boolean;
  blockedSessionOnly?: boolean;
  finalOnly?: boolean;
  includeSkills?: boolean;
  limit?: number;
};

export type ArtifactListOpts = {
  projectId?: string;
  sessionId?: string;
  kind?: string;
  excludeKind?: string;
  trustLevel?: string;
  unverifiedOnly?: boolean;
  blockedSessionOnly?: boolean;
  finalOnly?: boolean;
  limit?: number;
};

function assetQuery(opts?: AssetListOpts): string {
  const q = new URLSearchParams();
  if (opts?.projectId) q.set("project_id", opts.projectId);
  if (opts?.sessionId) q.set("session_id", opts.sessionId);
  if (opts?.assetKind) q.set("asset_kind", opts.assetKind);
  if (opts?.sourceType) q.set("source_type", opts.sourceType);
  if (opts?.reuseState) q.set("reuse_state", opts.reuseState);
  if (opts?.trustLevel) q.set("trust_level", opts.trustLevel);
  if (opts?.unverifiedOnly) q.set("unverified_only", "true");
  if (opts?.blockedSessionOnly) q.set("blocked_session_only", "true");
  if (opts?.finalOnly) q.set("final_only", "true");
  if (opts?.includeSkills === false) q.set("include_skills", "false");
  if (opts?.limit) q.set("limit", String(opts.limit));
  const qs = q.toString();
  return qs ? `?${qs}` : "";
}

function artifactQuery(opts?: ArtifactListOpts): string {
  const q = new URLSearchParams();
  if (opts?.projectId) q.set("project_id", opts.projectId);
  if (opts?.sessionId) q.set("session_id", opts.sessionId);
  if (opts?.kind) q.set("kind", opts.kind);
  if (opts?.excludeKind) q.set("exclude_kind", opts.excludeKind);
  if (opts?.trustLevel) q.set("trust_level", opts.trustLevel);
  if (opts?.unverifiedOnly) q.set("unverified_only", "true");
  if (opts?.blockedSessionOnly) q.set("blocked_session_only", "true");
  if (opts?.finalOnly) q.set("final_only", "true");
  if (opts?.limit) q.set("limit", String(opts.limit));
  const qs = q.toString();
  return qs ? `?${qs}` : "";
}

export const assetsClient = {
  assets: (opts?: AssetListOpts) =>
    get<{ assets: AssetItem[] }>(`/api/assets${assetQuery(opts)}`),

  assetDetail: (assetId: string) =>
    get<{ asset: AssetDetail }>(`/api/assets/${encodeURIComponent(assetId)}`),

  markAssetReusable: (assetId: string, body?: AssetActionRequest) =>
    post<AssetActionResult>(
      `/api/assets/${encodeURIComponent(assetId)}/mark-reusable`,
      body ?? {},
    ),

  archiveAsset: (assetId: string, body?: AssetActionRequest) =>
    post<AssetActionResult>(
      `/api/assets/${encodeURIComponent(assetId)}/archive`,
      body ?? {},
    ),

  promoteSkillDraft: (assetId: string) =>
    post<AssetActionResult>(
      `/api/assets/${encodeURIComponent(assetId)}/promote-skill-draft`,
      {},
    ),

  promoteWorkflowDraft: (assetId: string) =>
    post<AssetActionResult>(
      `/api/assets/${encodeURIComponent(assetId)}/promote-workflow-draft`,
      {},
    ),

  scanProjectWorkflows: (projectId: string) =>
    post<{ ok: boolean; result: { registered: number; paths: string[] } }>(
      `/api/projects/${encodeURIComponent(projectId)}/scan-workflows`,
      {},
    ),

  artifacts: (opts?: ArtifactListOpts) =>
    get<{ artifacts: ArtifactRecord[] }>(`/api/artifacts${artifactQuery(opts)}`),

  artifactDetail: (artifactId: string) =>
    get<{ artifact: ArtifactDetail }>(`/api/artifacts/${encodeURIComponent(artifactId)}`),

  indexProjectAssets: (projectId: string) =>
    post<{ ok: boolean; result: IndexAssetsResult }>(
      `/api/projects/${encodeURIComponent(projectId)}/index-assets`,
      {},
    ),
};
