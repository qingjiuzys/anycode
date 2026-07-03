/** Session id for which the global SSE hook should skip duplicate invalidation. */
let activeSessionId: string | null = null;

export function setActiveSessionForGlobalSse(sessionId: string | null): void {
  activeSessionId = sessionId;
}

export function getActiveSessionForGlobalSse(): string | null {
  return activeSessionId;
}
