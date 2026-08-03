import type {
  BrowserScreenshot,
  BrowserSessionInfo,
  BrowserState,
  FsEntry,
  FsReadResult,
  GitChangeKind,
  GitFileChange,
  GitFileDiff,
  GitStatusSummary,
  TerminalSessionInfo,
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

  createTerminalSession: (projectId: string, conversationId: string) =>
    post<{ session: TerminalSessionInfo }>("/api/workbench/terminal/sessions", {
      project_id: projectId,
      conversation_id: conversationId,
    }),

  listTerminalSessions: (projectId: string, conversationId: string) =>
    get<{ sessions: TerminalSessionInfo[] }>(
      `/api/workbench/terminal/sessions?project_id=${encodeURIComponent(projectId)}&conversation_id=${encodeURIComponent(conversationId)}`,
    ),

  deleteTerminalSession: (sessionId: string) =>
    del<{ ok: boolean }>(
      `/api/workbench/terminal/sessions/${encodeURIComponent(sessionId)}`,
    ),

  terminalSessionWsUrl: (sessionId: string) =>
    apiWebSocketUrl(
      `/api/workbench/terminal/sessions/${encodeURIComponent(sessionId)}/ws`,
    ),

  projectGitStatus: (projectId: string) =>
    get<{ git: GitStatusSummary }>(
      `/api/projects/${encodeURIComponent(projectId)}/git/status`,
    ),

  projectGitChanges: (projectId: string) =>
    get<{ changes: GitFileChange[] }>(
      `/api/projects/${encodeURIComponent(projectId)}/git/changes`,
    ),

  projectGitFileDiff: (projectId: string, path: string, kind: GitChangeKind) =>
    get<{ diff: GitFileDiff }>(
      `/api/projects/${encodeURIComponent(projectId)}/git/diff?path=${encodeURIComponent(path)}&kind=${encodeURIComponent(kind)}`,
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
