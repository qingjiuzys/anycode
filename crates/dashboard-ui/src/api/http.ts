export const API_BASE = import.meta.env.VITE_API_BASE ?? "";

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

export async function get<T>(path: string, opts?: { timeoutMs?: number }): Promise<T> {
  const res = await fetchWithTimeout(path, fetchOpts, opts?.timeoutMs ?? READ_TIMEOUT_MS);
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`${res.status} ${path}: ${body}`);
  }
  return readJsonBody<T>(res, path);
}

export async function post<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetchWithTimeout(path, {
    ...fetchOpts,
    method: "POST",
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  }, WRITE_TIMEOUT_MS);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${res.status} ${path}: ${text}`);
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
    throw new Error(`${res.status} ${path}: ${text}`);
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
    throw new Error(`${res.status} ${path}: ${text}`);
  }
  return readJsonBody<T>(res, path);
}

export async function del<T>(path: string): Promise<T> {
  const res = await fetchWithTimeout(path, { ...fetchOpts, method: "DELETE" }, READ_TIMEOUT_MS);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${res.status} ${path}: ${text}`);
  }
  return readJsonBody<T>(res, path);
}
