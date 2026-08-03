import type { PlanNode, PlanTree } from "@/api/types/workbench";

function storageKey(sessionId: string): string {
  return `anycode-plan-built:${sessionId}`;
}

export function readPlanBuiltAt(sessionId: string): string | null {
  if (typeof window === "undefined") return null;
  try {
    return localStorage.getItem(storageKey(sessionId));
  } catch {
    return null;
  }
}

export function markPlanBuilt(sessionId: string, updatedAt: string): void {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(storageKey(sessionId), updatedAt);
  } catch {
    /* ignore */
  }
}

function planNodeStarted(node: PlanNode): boolean {
  if (node.status === "in_progress" || node.status === "completed") {
    return true;
  }
  return (node.children ?? []).some(planNodeStarted);
}

export function planTreeExecutionStarted(tree: PlanTree): boolean {
  return tree.roots.some(planNodeStarted);
}

/** Plan exists and user has not confirmed Build for this revision. */
export function planAwaitingBuild(
  tree: PlanTree,
  updatedAt: string | null | undefined,
  sessionId: string,
): boolean {
  if (!updatedAt || tree.roots.length === 0) return false;
  if (readPlanBuiltAt(sessionId) === updatedAt) return false;
  return !planTreeExecutionStarted(tree);
}
