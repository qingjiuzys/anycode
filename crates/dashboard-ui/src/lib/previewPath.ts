import { projectFsRawUrl } from "@/lib/projectFsUrl";

/** Sidecar preview.html next to office deliverables (anycode-docx/xlsx/pdf skills). */
export function inferPreviewPath(path: string): string | null {
  const trimmed = path.trim();
  if (!trimmed) return null;
  const lower = trimmed.toLowerCase();
  if (lower.endsWith(".preview.html")) return trimmed;
  const dot = trimmed.lastIndexOf(".");
  if (dot <= 0) return null;
  const ext = lower.slice(dot + 1);
  if (
    !["docx", "doc", "xlsx", "xls", "pdf", "pptx", "ppt", "csv", "md", "markdown"].includes(
      ext,
    )
  ) {
    return null;
  }
  return `${trimmed.slice(0, dot)}.preview.html`;
}

export function resolvePreviewPath(
  path: string,
  previewPath?: string,
  previewSource: "self" | "sidecar" = "sidecar",
): string {
  if (previewSource === "self") return path;
  return previewPath?.trim() || inferPreviewPath(path) || path;
}

export function resolvePreviewUrl(
  projectId: string,
  path: string,
  previewPath?: string,
  previewSource: "self" | "sidecar" = "sidecar",
): string {
  return projectFsRawUrl(projectId, resolvePreviewPath(path, previewPath, previewSource));
}
