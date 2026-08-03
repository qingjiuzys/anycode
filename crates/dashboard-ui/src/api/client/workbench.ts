import type {
  BrowserScreenshot,
  BrowserSessionInfo,
  BrowserState,
  FsEntry,
  FsReadResult,
  GitStatusSummary,
} from "../types/workbench";
import { del, get, post, apiWebSocketUrl } from "../http";

export const workbenchClient = {
  listProjectFs: (projectId: string, path = "") =>
    get<{ entries: FsEntry[] }>(
      `/api/projects/${encodeURIComponent(projectId)}/fs/list?path=${encodeURIComponent(path)}`,
    ),

  readProjectFs: (projectId: string, path: string, maxBytes = 512 * 1024) =>
    get<{ file: FsReadResult }>(
      `/api/projects/${encodeURIComponent(projectId)}/fs/read?path=${encodeURIComponent(path)}&max_bytes=${maxBytes}`,
    ),

  createBrowserSession: (projectId: string, conversationId?: string | null) =>
    post<{ session: BrowserSessionInfo }>("/api/workbench/browser/sessions", {
      project_id: projectId,
      conversation_id: conversationId ?? undefined,
    }),

  navigateBrowser: (sessionId: string, url: string) =>
    post<{ state: BrowserState }>(
      `/api/workbench/browser/sessions/${encodeURIComponent(sessionId)}/navigate`,
      { url },
    ),

  browserState: (sessionId: string) =>
    get<{ state: BrowserState }>(
      `/api/workbench/browser/sessions/${encodeURIComponent(sessionId)}/state`,
    ),

  browserScreenshot: (sessionId: string) =>
    get<{ screenshot: BrowserScreenshot }>(
      `/api/workbench/browser/sessions/${encodeURIComponent(sessionId)}/screenshot`,
    ),

  deleteBrowserSession: (sessionId: string) =>
    del<{ ok: boolean }>(
      `/api/workbench/browser/sessions/${encodeURIComponent(sessionId)}`,
    ),

  browserLock: (sessionId: string, lock: "user" | "agent" | "idle") =>
    post<{ lock: string }>(
      `/api/workbench/browser/sessions/${encodeURIComponent(sessionId)}/lock`,
      { lock },
    ),

  browserStreamUrl: (sessionId: string) =>
    apiWebSocketUrl(
      `/api/workbench/browser/sessions/${encodeURIComponent(sessionId)}/stream`,
    ),

  browserStatus: () =>
    get<{
      enabled: boolean;
      ready: boolean;
      bundled: boolean;
      chromium_ready: boolean;
      doctor_message?: string;
    }>("/api/workbench/browser/status"),

  terminalWsUrl: (projectId: string) =>
    apiWebSocketUrl(`/api/projects/${encodeURIComponent(projectId)}/terminal/ws`),

  projectGitStatus: (projectId: string) =>
    get<{ git: GitStatusSummary }>(
      `/api/projects/${encodeURIComponent(projectId)}/git/status`,
    ),

  projectGitCommit: (projectId: string, body?: { message?: string }) =>
    post<{ ok: boolean }>(
      `/api/projects/${encodeURIComponent(projectId)}/git/commit`,
      body ?? {},
    ),

  projectGitPush: (projectId: string) =>
    post<{ ok: boolean; detail?: string }>(
      `/api/projects/${encodeURIComponent(projectId)}/git/push`,
      {},
    ),
};
