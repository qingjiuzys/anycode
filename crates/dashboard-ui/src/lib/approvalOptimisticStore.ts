/** Session-scoped optimistic approval resolutions (survives inbox remounts). */

type Listener = () => void;

const bySession = new Map<string, Set<string>>();
const listeners = new Set<Listener>();
let optimisticEpoch = 0;

function sessionKey(sessionId: string | undefined): string {
  return sessionId?.trim() || "__global__";
}

function emit(): void {
  optimisticEpoch += 1;
  for (const listener of listeners) {
    listener();
  }
}

export function markApprovalResolvedOptimistic(
  sessionId: string | undefined,
  approvalId: string,
): void {
  const id = approvalId.trim();
  if (!id) return;
  const key = sessionKey(sessionId);
  let set = bySession.get(key);
  if (!set) {
    set = new Set();
    bySession.set(key, set);
  }
  if (set.has(id)) return;
  set.add(id);
  emit();
}

export function unmarkApprovalResolvedOptimistic(
  sessionId: string | undefined,
  approvalId: string,
): void {
  const id = approvalId.trim();
  if (!id) return;
  const key = sessionKey(sessionId);
  const set = bySession.get(key);
  if (!set?.delete(id)) return;
  if (set.size === 0) bySession.delete(key);
  emit();
}

export function clearOptimisticResolvedApprovals(sessionId: string | undefined): void {
  const key = sessionKey(sessionId);
  if (!bySession.has(key)) return;
  bySession.delete(key);
  emit();
}

export function getOptimisticResolvedApprovalIds(
  sessionId: string | undefined,
): ReadonlySet<string> {
  return bySession.get(sessionKey(sessionId)) ?? EMPTY_SET;
}

const EMPTY_SET: ReadonlySet<string> = new Set();

export function subscribeOptimisticResolvedApprovals(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Snapshot string for useSyncExternalStore (stable when unchanged). */
export function optimisticResolvedApprovalsSnapshot(sessionId: string | undefined): string {
  const set = bySession.get(sessionKey(sessionId));
  if (!set || set.size === 0) return "";
  return [...set].sort().join("\0");
}

/** Global epoch — bumps on any session mark/unmark (for sidebar aggregate hooks). */
export function optimisticResolvedApprovalsEpoch(): number {
  return optimisticEpoch;
}
