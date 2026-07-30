import { isInlineKind, kindForPath, type ArtifactKind } from "@/lib/artifactKind";

export type ParsedArtifactMarker = {
  path: string;
  kind?: string;
  title?: string;
  inline?: boolean;
  mime?: string;
  previewPath?: string;
  bytes?: number;
};

const MARKER_PREFIX = "ANYCODE_ARTIFACT:";

function parseMarkerJson(jsonPart: string): ParsedArtifactMarker | null {
  try {
    const v = JSON.parse(jsonPart) as Record<string, unknown>;
    const path = typeof v.path === "string" ? v.path.trim() : "";
    if (!path) return null;
    return {
      path,
      kind: typeof v.kind === "string" ? v.kind : undefined,
      title: typeof v.title === "string" ? v.title : undefined,
      inline: typeof v.inline === "boolean" ? v.inline : undefined,
      mime: typeof v.mime === "string" ? v.mime : undefined,
      previewPath: typeof v.preview_path === "string" ? v.preview_path : undefined,
      bytes: typeof v.bytes === "number" ? v.bytes : undefined,
    };
  } catch {
    return null;
  }
}

/** Parse inline `ANYCODE_ARTIFACT:{...}` markers from assistant text. */
export function parseArtifactMarkers(text: string): ParsedArtifactMarker[] {
  const out: ParsedArtifactMarker[] = [];
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    const idx = trimmed.indexOf(MARKER_PREFIX);
    if (idx === -1) continue;
    const parsed = parseMarkerJson(trimmed.slice(idx + MARKER_PREFIX.length).trim());
    if (parsed) out.push(parsed);
  }
  return out;
}

function isArtifactScaffoldLine(line: string, markers: ParsedArtifactMarker[]): boolean {
  const trimmed = line.trim();
  if (!trimmed) return true;
  for (const marker of markers) {
    if (marker.title && trimmed.localeCompare(marker.title, undefined, { sensitivity: "accent" }) === 0) {
      return true;
    }
    const base = marker.path.split(/[/\\]/).pop();
    if (!base) continue;
    if (trimmed === base) return true;
    const baseNoExt = base.replace(/\.[^.]+$/, "");
    if (baseNoExt && trimmed.localeCompare(baseNoExt, undefined, { sensitivity: "accent" }) === 0) {
      return true;
    }
  }
  return false;
}

/** Remove artifact marker lines (and echoed absolute paths) from display text. */
export function stripArtifactMarkers(text: string): string {
  const markers = parseArtifactMarkers(text);
  const paths = new Set(markers.map((m) => m.path));
  const stripped = text
    .split("\n")
    .filter((line) => {
      const trimmed = line.trim();
      if (!trimmed) return true;
      if (trimmed.includes(MARKER_PREFIX)) return false;
      if (paths.has(trimmed)) return false;
      if (markers.length > 0 && isArtifactScaffoldLine(trimmed, markers)) return false;
      return true;
    })
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
  return stripped;
}

export function markerShouldInline(marker: ParsedArtifactMarker): boolean {
  if (marker.inline === false) return false;
  if (marker.inline === true) return true;
  const kind = kindForPath(marker.path, marker.kind) as ArtifactKind;
  return isInlineKind(kind);
}

/** Standalone echo lines agents emit before artifact markers (often persisted without the marker). */
export function isArtifactScaffoldOnly(text: string): boolean {
  const trimmed = text.trim();
  if (!trimmed) return true;
  if (/^anycode$/i.test(trimmed)) return true;
  if (/^[\w./\\-]+\.(md|markdown|html|pdf|pptx?|xlsx?|json)$/i.test(trimmed)) return true;
  return false;
}
