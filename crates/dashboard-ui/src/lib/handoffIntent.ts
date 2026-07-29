import type { HandoffKind } from "@/api/client/lan";
import { buildControlCenterHref } from "@/lib/controlCenterPaths";

export type HandoffIntent = {
  kind: HandoffKind;
  projectId: string;
  sessionId?: string;
};

export function parseHandoffIntent(
  search: Record<string, string> | undefined,
): HandoffIntent | null {
  if (!search) return null;
  const kind = search.handoff;
  if (kind !== "project" && kind !== "session") return null;
  const projectId = search.projectId?.trim();
  if (!projectId) return null;
  const sessionId = search.sessionId?.trim();
  if (kind === "session" && !sessionId) return null;
  return { kind, projectId, sessionId };
}

export function buildHandoffColleaguesPath(intent: HandoffIntent): string {
  return buildControlCenterHref("/colleagues", undefined, {
    handoff: intent.kind,
    projectId: intent.projectId,
    sessionId: intent.sessionId,
  });
}
