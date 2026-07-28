import { get, patch, post } from "../http";

export type HandoffKind = "project" | "session";

export type HandoffState =
  | "pending_approval"
  | "approved"
  | "uploading"
  | "importing"
  | "completed"
  | "rejected"
  | "failed"
  | "expired";

export interface LanPeer {
  instance_id: string;
  device_name: string;
  host: string;
  lan_port: number;
  version: string;
  last_seen: string;
}

export interface LanSettings {
  discovery_enabled: boolean;
  display_name: string;
  lan_port: number;
  max_bundle_mb: number;
}

export interface HandoffRecord {
  id: string;
  kind: HandoffKind;
  state: HandoffState;
  direction: "outgoing" | "incoming";
  sender: { instance_id: string; device_name: string; host: string; lan_port: number };
  recipient: { instance_id: string; device_name: string; host: string; lan_port: number };
  project_id?: string | null;
  project_name?: string | null;
  session_id?: string | null;
  session_title?: string | null;
  target_project_id?: string | null;
  target_root_path?: string | null;
  progress_pct?: number;
  error?: string | null;
}

export interface OutgoingHandoffStatus {
  id: string;
  state: HandoffState;
  progress_pct: number;
  error?: string | null;
}

export const lanClient = {
  listPeers: () =>
    get<{
      peers: LanPeer[];
      enabled: boolean;
      instance_id?: string;
      display_name?: string;
    }>("/api/lan/peers"),

  getSettings: () => get<{ settings: LanSettings }>("/api/lan/settings"),

  patchSettings: (body: Partial<LanSettings>) =>
    patch<{ ok: boolean; settings: LanSettings }>("/api/lan/settings", body),

  requestHandoff: (body: {
    peer_id: string;
    kind: HandoffKind;
    project_id?: string;
    session_id?: string;
    target_project_id?: string;
  }) => post<{ ok: boolean; handoff_id: string }>("/api/lan/handoff/request", body),

  listIncoming: () => get<{ requests: HandoffRecord[] }>("/api/lan/handoff/incoming"),

  listOutgoing: () => get<{ requests: OutgoingHandoffStatus[] }>("/api/lan/handoff/outgoing"),

  approveHandoff: (
    handoffId: string,
    body: { target_root_path?: string; target_project_id?: string },
  ) => post<{ ok: boolean }>(`/api/lan/handoff/${encodeURIComponent(handoffId)}/approve`, body),

  rejectHandoff: (handoffId: string) =>
    post<{ ok: boolean }>(`/api/lan/handoff/${encodeURIComponent(handoffId)}/reject`, {}),
};
