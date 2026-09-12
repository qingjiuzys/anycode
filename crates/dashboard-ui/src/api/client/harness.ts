import { get, post } from "../http";
import { currentHarnessDeviceToken } from "@/lib/desktopShell";
import type { Graph, Checkpoint } from "@/features/harness/graph-model.mjs";

export type HarnessStatus = {
  graph: boolean;
  gray_projects: string[];
  unified_kernel: boolean;
  computer: boolean;
  x11_verified: boolean;
  macos_screencapture?: boolean;
  accounts_sso: boolean;
  pairing: "local" | "accounts_sso";
  run_store: "file" | string;
};

export type HarnessGraphResult = {
  status: string;
  run_id?: string;
  revision?: number;
  checkpoint?: Checkpoint | null;
  graph?: Graph;
  error?: string;
};

export type HarnessPairingChallenge = {
  device_id: string;
  challenge: string;
  expires_in: number;
};

export type HarnessPairingConfirm = {
  device_id: string;
  token: string;
  keychain_service: string;
  keychain_account: string;
};

async function harnessDeviceHeaders(): Promise<Record<string, string>> {
  const token = await currentHarnessDeviceToken();
  return token ? { "x-anycode-device-token": token } : {};
}

export const harnessClient = {
  harnessStatus: () => get<HarnessStatus>("/api/harness/status"),
  harnessGraph: async (projectId: string, runId: string) =>
    get<HarnessGraphResult>(
      `/api/projects/${encodeURIComponent(projectId)}/harness/graphs/${encodeURIComponent(runId)}`,
      { headers: await harnessDeviceHeaders() },
    ),
  harnessGraphStart: async (projectId: string, graph: Graph, sessionId?: string) =>
    post<HarnessGraphResult>(
      `/api/projects/${encodeURIComponent(projectId)}/harness/graphs/start`,
      { graph, session_id: sessionId },
      { timeoutMs: 120_000, headers: await harnessDeviceHeaders() },
    ),
  harnessGraphResume: async (projectId: string, runId: string, sessionId?: string) =>
    post<HarnessGraphResult>(
      `/api/projects/${encodeURIComponent(projectId)}/harness/graphs/${encodeURIComponent(runId)}/resume`,
      { session_id: sessionId },
      { timeoutMs: 120_000, headers: await harnessDeviceHeaders() },
    ),
  harnessGraphResolve: async (
    projectId: string,
    runId: string,
    body: { revision: number; node_id: string; approved: boolean; session_id?: string },
  ) =>
    post<HarnessGraphResult>(
      `/api/projects/${encodeURIComponent(projectId)}/harness/graphs/${encodeURIComponent(runId)}/resolve`,
      body,
      { headers: await harnessDeviceHeaders() },
    ),
  harnessPairingChallenge: (label = "desktop") =>
    post<HarnessPairingChallenge>("/api/harness/pairing/challenges", { label }),
  harnessPairingConfirm: (challenge: string) =>
    post<HarnessPairingConfirm>("/api/harness/pairing/confirm", { challenge }),
  harnessPairingRevoke: (deviceId: string) =>
    post<{ ok: boolean; revoked: boolean }>("/api/harness/pairing/revoke", {
      device_id: deviceId,
    }),
  harnessComputerTicket: async (projectId: string, runId: string, ttlSecs = 60) =>
    post<{
      ticket: string;
      run_id: string;
      device: string;
      backend: string;
      x11_verified: boolean;
    }>(
      `/api/projects/${encodeURIComponent(projectId)}/harness/computer/tickets`,
      { run_id: runId, ttl_secs: ttlSecs },
      { headers: await harnessDeviceHeaders() },
    ),
};
