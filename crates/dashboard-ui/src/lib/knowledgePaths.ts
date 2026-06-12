export const DEFAULT_KNOWLEDGE_PATHS = ["docs/", "reports/", "references/"];

export function parseKnowledgePaths(text: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || seen.has(trimmed)) continue;
    seen.add(trimmed);
    out.push(trimmed);
  }
  return out;
}

export function validateKnowledgePath(path: string): string | null {
  const trimmed = path.trim();
  if (!trimmed) return "empty";
  if (trimmed.startsWith("/") || /^[a-zA-Z]:/.test(trimmed)) {
    return "absolute";
  }
  if (trimmed.includes("..")) {
    return "parent";
  }
  return null;
}

export function pathsEqual(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return false;
  const sortedA = [...a].sort();
  const sortedB = [...b].sort();
  return sortedA.every((v, i) => v === sortedB[i]);
}

export type KnowledgeIndexStatus = "empty" | "stale" | "ready";

export function knowledgeIndexStatus(
  paths: string[],
  savedPaths: string[],
  chunkCount: number,
): KnowledgeIndexStatus {
  if (paths.length === 0) return "empty";
  if (!pathsEqual(paths, savedPaths)) return "stale";
  if (chunkCount === 0) return "stale";
  return "ready";
}
