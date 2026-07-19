/** Dashboard API helpers for e2e delivery chain. */

const DEFAULT_BASE = process.env.ANYCODE_E2E_BASE ?? "http://127.0.0.1:43180";

export function apiBase() {
  return DEFAULT_BASE.replace(/\/$/, "");
}

export async function health() {
  const res = await fetch(`${apiBase()}/api/health`);
  if (!res.ok) throw new Error(`health ${res.status}`);
  return res.json();
}

export async function upsertProject(body) {
  const res = await fetch(`${apiBase()}/api/projects`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const data = await res.json();
  if (!res.ok) throw new Error(data.error ?? `upsert project ${res.status}`);
  return data.project;
}

export async function startConversation(projectId, prompt, agent = "office-writer") {
  const res = await fetch(`${apiBase()}/api/projects/${encodeURIComponent(projectId)}/conversations/start`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ prompt, agent, recycle_session: false }),
  });
  const data = await res.json();
  if (!res.ok) throw new Error(data.error ?? `start conversation ${res.status}`);
  return data;
}

export async function getSession(sessionId) {
  const res = await fetch(`${apiBase()}/api/sessions/${encodeURIComponent(sessionId)}`);
  const data = await res.json();
  if (!res.ok) throw new Error(data.error ?? `get session ${res.status}`);
  return data.session;
}

export async function getTranscript(sessionId) {
  const res = await fetch(`${apiBase()}/api/sessions/${encodeURIComponent(sessionId)}/transcript`);
  const data = await res.json();
  if (!res.ok) throw new Error(data.error ?? `transcript ${res.status}`);
  return data.transcript;
}

export async function waitForSession(sessionId, { timeoutMs = 600_000, pollMs = 3_000 } = {}) {
  const startedAt = Date.now();
  const deadline = Date.now() + timeoutMs;
  let nextProgressAt = startedAt + 30_000;
  while (Date.now() < deadline) {
    const session = await getSession(sessionId);
    const st = session.status;
    if (st === "completed" || st === "failed" || st === "cancelled") {
      return session;
    }
    if (Date.now() >= nextProgressAt) {
      const elapsedSeconds = Math.round((Date.now() - startedAt) / 1000);
      console.log(`[session-progress] id=${sessionId} status=${st} elapsed=${elapsedSeconds}s`);
      nextProgressAt = Date.now() + 30_000;
    }
    await new Promise((r) => setTimeout(r, pollMs));
  }
  throw new Error(`session ${sessionId} timed out after ${timeoutMs}ms`);
}
