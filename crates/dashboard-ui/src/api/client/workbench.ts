import type {
  BrowserScreenshot,
  BrowserSessionInfo,
  BrowserState,
  FsEntry,
  FsReadResult,
} from "../types/workbench";
import { del, get, post } from "../http";

export const workbenchClient = {
  listProjectFs: (projectId: string, path = "") =>
    get<{ entries: FsEntry[] }>(
      `/api/projects/${encodeURIComponent(projectId)}/fs/list?path=${encodeURIComponent(path)}`,
    ),

  readProjectFs: (projectId: string, path: string, maxBytes = 512 * 1024) =>
    get<{ file: FsReadResult }>(
      `/api/projects/${encodeURIComponent(projectId)}/fs/read?path=${encodeURIComponent(path)}&max_bytes=${maxBytes}`,
    ),

  createBrowserSession: (projectId: string) =>
    post<{ session: BrowserSessionInfo }>("/api/workbench/browser/sessions", {
      project_id: projectId,
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

  terminalWsUrl: (projectId: string) => {
    const base = import.meta.env.VITE_API_BASE ?? "";
    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    const host = base
      ? new URL(base, window.location.origin).host
      : window.location.host;
    return `${proto}//${host}/api/projects/${encodeURIComponent(projectId)}/terminal/ws`;
  },
};
