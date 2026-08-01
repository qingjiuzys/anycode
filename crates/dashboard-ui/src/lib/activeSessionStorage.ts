const ACTIVE_SESSION_KEY = "anycode-active-session";

export function readPinnedSessionId(): string | null {
  try {
    const id = sessionStorage.getItem(ACTIVE_SESSION_KEY)?.trim();
    return id || null;
  } catch {
    return null;
  }
}

export function writePinnedSessionId(sessionId: string | null): void {
  try {
    if (sessionId?.trim()) {
      sessionStorage.setItem(ACTIVE_SESSION_KEY, sessionId.trim());
    } else {
      sessionStorage.removeItem(ACTIVE_SESSION_KEY);
    }
  } catch {
    /* ignore quota / private mode */
  }
}

/** Resolve which session the shell should bind to from URL + sidebar pool + pin. */
export function resolveShellSessionId(input: {
  pathname: string;
  urlSession: string | undefined;
  pinnedSessionId: string | null;
  fallbackSessionId: string | null;
}): string | null {
  if (input.urlSession?.trim()) {
    return input.urlSession.trim();
  }
  if (input.pinnedSessionId?.trim()) {
    return input.pinnedSessionId.trim();
  }
  if (input.pathname === "/") {
    return null;
  }
  return input.fallbackSessionId;
}
