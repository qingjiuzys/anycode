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
    return await fetch(`${API_BASE}${path}`, {
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

export async function get<T>(path: string): Promise<T> {
  const res = await fetchWithTimeout(path, fetchOpts, READ_TIMEOUT_MS);
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`${res.status} ${path}: ${body}`);
  }
  return res.json() as Promise<T>;
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
  return res.json() as Promise<T>;
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
  return res.json() as Promise<T>;
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
  return res.json() as Promise<T>;
}

export async function del<T>(path: string): Promise<T> {
  const res = await fetchWithTimeout(path, { ...fetchOpts, method: "DELETE" }, WRITE_TIMEOUT_MS);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${res.status} ${path}: ${text}`);
  }
  return res.json() as Promise<T>;
}
