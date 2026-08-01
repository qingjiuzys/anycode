/** True when path looks absolute (POSIX or Windows). */
export function isAbsolutePath(path: string): boolean {
  const p = path.trim();
  if (!p) return false;
  if (p.startsWith("/")) return true;
  // Windows: C:\ or \\server\share
  if (/^[A-Za-z]:[\\/]/.test(p)) return true;
  if (p.startsWith("\\\\")) return true;
  return false;
}

/**
 * Resolve a deliverable path against the project root.
 * Absolute paths are returned unchanged; relative paths join under root.
 */
export function resolveDeliverableAbsPath(
  path: string,
  projectRoot?: string | null,
): string {
  const trimmed = path.trim();
  if (!trimmed) return "";
  if (isAbsolutePath(trimmed)) return trimmed;
  const root = (projectRoot ?? "").trim().replace(/[/\\]+$/, "");
  if (!root) return trimmed;
  const rel = trimmed.replace(/^\.([/\\])/, "");
  const sep = root.includes("\\") && !root.includes("/") ? "\\" : "/";
  return `${root}${sep}${rel.replace(/^[/\\]+/, "")}`;
}

/** Intermediate / draft filenames that should not clutter the deliverable strip. */
export function isProcessArtifactPath(path: string): boolean {
  const base = path.split(/[/\\]/).pop()?.toLowerCase() ?? path.toLowerCase();
  if (base.includes(".anycode-artifact")) return true;
  if (/(?:^|[-_])(tmp|temp|scratch|wip|draft)(?:[-_.]|$)/i.test(base)) return true;
  // Iterative trial suffixes (keep intentional names like *-complex.md).
  if (/-(?:deep|depth|draft|v\d+)(?:\.|-)/i.test(base)) return true;
  return false;
}

/** High-value kinds shown as conversation deliverables (not every written .md). */
export function isPrimaryDeliverableKind(kind: string | undefined): boolean {
  return (
    kind === "mindmap" ||
    kind === "report" ||
    kind === "spreadsheet" ||
    kind === "presentation" ||
    kind === "pdf" ||
    kind === "image" ||
    kind === "video" ||
    kind === "audio" ||
    kind === "media"
  );
}
