import type { GateExecuteResult } from "@/api/types";

const API_BASE = import.meta.env.VITE_API_BASE ?? "";

export type GateStreamEvent =
  | { type: "line"; line: string }
  | { type: "done"; result: GateExecuteResult }
  | { type: "error"; error: string };

/** POST gate execute with SSE line streaming. */
export async function streamGateExecute(
  projectId: string,
  body: { preset_id?: string; name?: string; command?: string; required?: boolean },
  onEvent: (ev: GateStreamEvent) => void,
  signal?: AbortSignal,
): Promise<void> {
  const res = await fetch(
    `${API_BASE}/api/projects/${encodeURIComponent(projectId)}/gates/execute/stream`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "text/event-stream" },
      body: JSON.stringify(body),
      signal,
    },
  );
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(String((err as { error?: string }).error ?? res.statusText));
  }
  if (!res.body) {
    throw new Error("No response body");
  }
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    const parts = buffer.split("\n\n");
    buffer = parts.pop() ?? "";
    for (const part of parts) {
      const dataLine = part.split("\n").find((l) => l.startsWith("data:"));
      if (!dataLine) continue;
      const raw = dataLine.slice(5).trim();
      try {
        const parsed = JSON.parse(raw) as GateStreamEvent;
        onEvent(parsed);
      } catch {
        // ignore malformed chunks
      }
    }
  }
}
