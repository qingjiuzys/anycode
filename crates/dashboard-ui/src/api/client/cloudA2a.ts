import { get, post } from "../http";

export type CloudTeamPeer = {
  user_id: string;
  display_name: string;
  email: string;
  device_id: string;
  instance_id: string;
  device_name: string;
  version: string;
  transport: string;
  online: boolean;
  last_seen: string;
  capabilities: string[];
};

export type CloudHandoffTask = {
  id: string;
  kind: "project" | "session";
  state: string;
  sender_name: string;
  recipient_name: string;
  project_id?: string;
  project_name?: string;
  session_id?: string;
  session_title?: string;
  progress_pct: number;
};

export const cloudA2aClient = {
  listTeamPeers: () => get<{ peers: CloudTeamPeer[] }>("/api/cloud/a2a/team/peers"),

  requestCloudHandoff: (body: {
    recipient_device_id: string;
    recipient_instance_id: string;
    kind: "project" | "session";
    project_id?: string;
    session_id?: string;
    target_project_id?: string;
  }) => post<{ handoff: CloudHandoffTask }>("/api/cloud/a2a/handoff/request", body),

  listCloudIncoming: () =>
    get<{ incoming: CloudHandoffTask[] }>("/api/cloud/a2a/handoff/incoming"),

  listCloudOutgoing: () =>
    get<{ outgoing: CloudHandoffTask[] }>("/api/cloud/a2a/handoff/outgoing"),

  approveCloudHandoff: (handoffId: string, body: { target_project_id?: string }) =>
    post<{ handoff: CloudHandoffTask }>(
      `/api/cloud/a2a/handoff/${encodeURIComponent(handoffId)}/approve`,
      body,
    ),

  rejectCloudHandoff: (handoffId: string) =>
    post<{ handoff: CloudHandoffTask }>(
      `/api/cloud/a2a/handoff/${encodeURIComponent(handoffId)}/reject`,
      {},
    ),
};
