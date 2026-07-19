declare global {
  interface Window {
    __ANYCODE_API_BASE__?: string;
  }
}

/** Loopback API origin when UI is loaded outside the dashboard HTTP server (e.g. Tauri asset). */
export function resolveApiBase(): string {
  if (typeof window !== "undefined") {
    if (window.__ANYCODE_API_BASE__) {
      return window.__ANYCODE_API_BASE__.replace(/\/$/, "");
    }
    try {
      const stored = sessionStorage.getItem("anycode_api_base");
      if (stored) return stored.replace(/\/$/, "");
    } catch {
      /* private mode / disabled storage */
    }
    if ("__TAURI_INTERNALS__" in window) {
      return "http://127.0.0.1:43180";
    }
  }
  const fromEnv = import.meta.env.VITE_API_BASE ?? "";
  return fromEnv.replace(/\/$/, "");
}

export const API_BASE = resolveApiBase();

const fetchOpts: RequestInit = { credentials: "include" };
const READ_TIMEOUT_MS = 15_000;
const WRITE_TIMEOUT_MS = 30_000;

async function fetchWithTimeout(
  path: string,
  init: RequestInit,
  timeoutMs: number,
): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const url =
      API_BASE !== ""
        ? new URL(path, API_BASE.endsWith("/") ? API_BASE : `${API_BASE}/`).href
        : path;
    return await fetch(url, {
      ...init,
      signal: controller.signal,
    });
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") {
      throw new Error(`Request timed out after ${Math.round(timeoutMs / 1000)}s: ${path}`);
    }
    throw error;
  } finally {
    clearTimeout(timer);
  }
}

async function readJsonBody<T>(res: Response, path: string): Promise<T> {
  const text = await res.text();
  if (!text.trim()) {
    throw new Error(`${res.status} ${path}: empty response body`);
  }
  const ct = res.headers.get("content-type") ?? "";
  if (!ct.includes("json") && text.trimStart().startsWith("<")) {
    throw new Error(
      `${res.status} ${path}: expected JSON but got HTML (restart Workbench or update anycode)`,
    );
  }
  try {
    return JSON.parse(text) as T;
  } catch {
    throw new Error(`${res.status} ${path}: invalid JSON (${text.slice(0, 160)})`);
  }
}

export function parseApiErrorBody(text: string): string {
  try {
    const parsed = JSON.parse(text) as { error?: unknown; message?: unknown };
    if (typeof parsed.error === "string" && parsed.error.trim()) {
      return parsed.error;
    }
    if (typeof parsed.message === "string" && parsed.message.trim()) {
      return parsed.message;
    }
  } catch {
    /* not JSON */
  }
  return text;
}

export async function get<T>(path: string, opts?: { timeoutMs?: number }): Promise<T> {
  const res = await fetchWithTimeout(path, fetchOpts, opts?.timeoutMs ?? READ_TIMEOUT_MS);
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`${res.status} ${path}: ${parseApiErrorBody(body)}`);
  }
  return readJsonBody<T>(res, path);
}

export async function post<T>(
  path: string,
  body?: unknown,
  opts?: { timeoutMs?: number; acceptStatuses?: number[] },
): Promise<T> {
  const res = await fetchWithTimeout(path, {
    ...fetchOpts,
    method: "POST",
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  }, opts?.timeoutMs ?? WRITE_TIMEOUT_MS);
  const accept = opts?.acceptStatuses ?? [];
  if (!res.ok && !accept.includes(res.status)) {
    const text = await res.text();
    throw new Error(`${res.status} ${path}: ${parseApiErrorBody(text)}`);
  }
  return readJsonBody<T>(res, path);
}

export async function put<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetchWithTimeout(path, {
    ...fetchOpts,
    method: "PUT",
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  }, WRITE_TIMEOUT_MS);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${res.status} ${path}: ${parseApiErrorBody(text)}`);
  }
  return readJsonBody<T>(res, path);
}

export async function patch<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetchWithTimeout(path, {
    ...fetchOpts,
    method: "PATCH",
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  }, WRITE_TIMEOUT_MS);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${res.status} ${path}: ${parseApiErrorBody(text)}`);
  }
  return readJsonBody<T>(res, path);
}

export async function del<T>(path: string): Promise<T> {
  const res = await fetchWithTimeout(path, { ...fetchOpts, method: "DELETE" }, READ_TIMEOUT_MS);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${res.status} ${path}: ${parseApiErrorBody(text)}`);
  }
  return readJsonBody<T>(res, path);
}

/** Build an absolute API URL (SSE, WebSocket, downloads). */
export function apiUrl(path: string): string {
  if (API_BASE !== "") {
    return new URL(path, `${API_BASE}/`).href;
  }
  return path;
}

/** WebSocket URL for a path under `/api/`. */
export function apiWebSocketUrl(path: string): string {
  if (API_BASE !== "") {
    const http = new URL(API_BASE);
    const wsProto = http.protocol === "https:" ? "wss:" : "ws:";
    return `${wsProto}//${http.host}${path.startsWith("/") ? path : `/${path}`}`;
  }
  const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${proto}//${window.location.host}${path.startsWith("/") ? path : `/${path}`}`;
}
