export interface AuthUser {
  id: string;
  email: string;
  display_name: string;
  role: string;
  organization_id: string;
  auth_method: string;
}

export interface SessionListOpts {
  limit?: number;
  kind?: string;
  status?: string;
  trustedStatus?: string;
  projectId?: string;
  budgetExceeded?: boolean;
}

export interface ProjectsListOpts {
  limit?: number;
  offset?: number;
  q?: string;
  status?: string;
  sort?: "updated_at_desc" | "updated_at_asc" | "name_asc" | "name_desc" | "sessions_desc";
}

export interface EventListOpts {
  eventType?: string;
  severity?: string;
  q?: string;
  limit?: number;
}

export interface ArtifactListOpts {
  projectId?: string;
  sessionId?: string;
  kind?: string;
  trustLevel?: string;
  unverifiedOnly?: boolean;
  blockedSessionOnly?: boolean;
  limit?: number;
}

export function buildArtifactQuery(opts?: ArtifactListOpts): URLSearchParams {
  const q = new URLSearchParams();
  q.set("limit", String(opts?.limit ?? 100));
  if (opts?.projectId) q.set("project_id", opts.projectId);
  if (opts?.sessionId) q.set("session_id", opts.sessionId);
  if (opts?.kind) q.set("kind", opts.kind);
  if (opts?.trustLevel) q.set("trust_level", opts.trustLevel);
  if (opts?.unverifiedOnly) q.set("unverified_only", "true");
  if (opts?.blockedSessionOnly) q.set("blocked_session_only", "true");
  return q;
}
