import type { ChatStreamEvent } from "@/lib/liveTranscript";

export type SessionStreamSnapshot = {
  liveEvents: ChatStreamEvent[];
  lastSeq: number;
  chatLive: boolean;
};

type Listener = () => void;

const bySession = new Map<string, SessionStreamSnapshot>();
const listeners = new Set<Listener>();

function emptySnapshot(): SessionStreamSnapshot {
  return { liveEvents: [], lastSeq: 0, chatLive: false };
}

function emit(): void {
  for (const listener of listeners) {
    listener();
  }
}

export function getSessionStreamSnapshot(sessionId: string | undefined): SessionStreamSnapshot {
  const id = sessionId?.trim();
  if (!id) return emptySnapshot();
  return bySession.get(id) ?? emptySnapshot();
}

export function setSessionStreamSnapshot(
  sessionId: string | undefined,
  snapshot: SessionStreamSnapshot,
): void {
  const id = sessionId?.trim();
  if (!id) return;
  if (snapshot.liveEvents.length === 0 && !snapshot.chatLive && snapshot.lastSeq === 0) {
    if (!bySession.has(id)) return;
    bySession.delete(id);
    emit();
    return;
  }
  bySession.set(id, {
    liveEvents: snapshot.liveEvents,
    lastSeq: snapshot.lastSeq,
    chatLive: snapshot.chatLive,
  });
  emit();
}

export function clearSessionStreamSnapshot(sessionId: string | undefined): void {
  const id = sessionId?.trim();
  if (!id || !bySession.has(id)) return;
  bySession.delete(id);
  emit();
}

export function subscribeSessionStreamStore(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
