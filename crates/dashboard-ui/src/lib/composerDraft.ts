const DRAFT_PREFIX = "anycode.composer.draft:";

/** Per-session composer draft cache key. `scope` = session id or `project:<id>`. */
export function composerDraftKey(scope: string | undefined): string | null {
  if (!scope) return null;
  return `${DRAFT_PREFIX}${scope}`;
}

/** Restore a previously typed (unsent) composer draft for this session scope. */
export function loadComposerDraft(scope: string | undefined): string {
  const key = composerDraftKey(scope);
  if (!key || typeof sessionStorage === "undefined") return "";
  try {
    return sessionStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

/** Persist the current composer draft so switching away does not lose it. */
export function saveComposerDraft(scope: string | undefined, text: string): void {
  const key = composerDraftKey(scope);
  if (!key || typeof sessionStorage === "undefined") return;
  try {
    if (text.trim().length === 0) {
      sessionStorage.removeItem(key);
    } else {
      sessionStorage.setItem(key, text);
    }
  } catch {
    /* private mode / quota */
  }
}

/** Clear a session draft after it has been sent (or explicitly discarded). */
export function clearComposerDraft(scope: string | undefined): void {
  const key = composerDraftKey(scope);
  if (!key || typeof sessionStorage === "undefined") return;
  try {
    sessionStorage.removeItem(key);
  } catch {
    /* ignore */
  }
}