import type {
  QuickAuthPreset,
  SetupStatus,
} from "../types/setup";
import { get, patch, post } from "../http";

export const setupClient = {
  setupStatus: () => get<{ setup: SetupStatus }>("/api/setup/status"),
  setupQuickAuth: () =>
    get<{ presets: QuickAuthPreset[] }>("/api/setup/quick-auth"),
  setupEnsureWorkspace: () =>
    post<{ ok: boolean; error?: string }>("/api/setup/workspace/ensure", {}),
  setupMemory: (body: {
    preset: string;
    embedding_base_url?: string;
    embedding_model?: string;
  }) => patch<{ ok: boolean; config_path?: string; error?: string }>("/api/setup/memory", body),
  setupComplete: (body?: { scan_projects?: boolean }) =>
    post<{ ok: boolean; setup_completed_at?: string; error?: string }>(
      "/api/setup/complete",
      body ?? { scan_projects: true },
    ),
};
