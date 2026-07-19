const STORAGE_KEY = "anycode-pinned-projects";

export function readPinnedProjectIds(): string[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((id): id is string => typeof id === "string" && id.length > 0);
  } catch {
    return [];
  }
}

export function writePinnedProjectIds(ids: string[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify([...new Set(ids)]));
}

export function togglePinnedProjectId(projectId: string): string[] {
  const current = readPinnedProjectIds();
  const next = current.includes(projectId)
    ? current.filter((id) => id !== projectId)
    : [projectId, ...current];
  writePinnedProjectIds(next);
  return next;
}
