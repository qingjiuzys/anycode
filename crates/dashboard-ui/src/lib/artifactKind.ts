export type ArtifactKind =
  | "image"
  | "video"
  | "audio"
  | "pdf"
  | "presentation"
  | "document"
  | "mindmap"
  | "report"
  | "file"
  | "media";

const IMAGE_EXT = new Set(["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "avif"]);
const VIDEO_EXT = new Set(["mp4", "mov", "webm", "mkv", "m4v", "avi"]);
const AUDIO_EXT = new Set(["mp3", "wav", "ogg", "m4a", "aac", "flac"]);
const PRESENTATION_EXT = new Set(["pptx", "ppt", "key", "odp"]);
const DOCUMENT_EXT = new Set(["docx", "doc", "xlsx", "xls", "csv", "txt", "rtf", "odt", "ods"]);
const REPORT_EXT = new Set(["html", "htm"]);
const MINDMAP_EXT = new Set(["mm", "xmind"]);

function extension(path: string): string {
  const base = path.split(/[/\\]/).pop() ?? path;
  const dot = base.lastIndexOf(".");
  if (dot <= 0) return "";
  return base.slice(dot + 1).toLowerCase();
}

function normalizeKind(value: string | undefined | null): ArtifactKind | null {
  if (!value) return null;
  const lower = value.toLowerCase();
  const kinds: ArtifactKind[] = [
    "image",
    "video",
    "audio",
    "pdf",
    "presentation",
    "document",
    "mindmap",
    "report",
    "file",
    "media",
  ];
  if (kinds.includes(lower as ArtifactKind)) {
    return lower as ArtifactKind;
  }
  if (lower.includes("presentation") || lower.includes("slideshow")) return "presentation";
  if (lower.includes("document")) return "document";
  if (lower.includes("mindmap") || lower.includes("mind-map")) return "mindmap";
  if (lower.includes("report")) return "report";
  if (lower.includes("image")) return "image";
  if (lower.includes("video")) return "video";
  if (lower.includes("audio")) return "audio";
  if (lower.includes("media")) return "media";
  return null;
}

export function kindForPath(path: string, hint?: string | null): ArtifactKind {
  const fromHint = normalizeKind(hint);
  if (fromHint && fromHint !== "file" && fromHint !== "media") {
    return fromHint;
  }

  const ext = extension(path);
  const lowerPath = path.toLowerCase();

  if (ext === "pdf") return "pdf";
  if (IMAGE_EXT.has(ext)) return "image";
  if (VIDEO_EXT.has(ext)) return "video";
  if (AUDIO_EXT.has(ext)) return "audio";
  if (PRESENTATION_EXT.has(ext)) return "presentation";
  if (MINDMAP_EXT.has(ext) || lowerPath.includes("mindmap") || lowerPath.includes("mind-map")) {
    return "mindmap";
  }
  if (ext === "md" && (lowerPath.includes("mindmap") || lowerPath.includes("mind-map"))) {
    return "mindmap";
  }
  if (REPORT_EXT.has(ext)) return "report";
  if (DOCUMENT_EXT.has(ext) || ext === "md") return "document";

  if (fromHint) return fromHint;
  return "file";
}

export function mimeForPath(path: string, hint?: string | null): string {
  if (hint?.includes("/")) return hint;

  const ext = extension(path);
  const map: Record<string, string> = {
    png: "image/png",
    jpg: "image/jpeg",
    jpeg: "image/jpeg",
    gif: "image/gif",
    webp: "image/webp",
    svg: "image/svg+xml",
    bmp: "image/bmp",
    avif: "image/avif",
    mp4: "video/mp4",
    mov: "video/quicktime",
    webm: "video/webm",
    mkv: "video/x-matroska",
    mp3: "audio/mpeg",
    wav: "audio/wav",
    ogg: "audio/ogg",
    m4a: "audio/mp4",
    pdf: "application/pdf",
    pptx: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    ppt: "application/vnd.ms-powerpoint",
    docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    doc: "application/msword",
    xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    xls: "application/vnd.ms-excel",
    csv: "text/csv",
    txt: "text/plain",
    md: "text/markdown",
    html: "text/html",
    htm: "text/html",
    json: "application/json",
  };
  return map[ext] ?? "application/octet-stream";
}

export function isInlineKind(kind: ArtifactKind): boolean {
  return (
    kind === "image" ||
    kind === "video" ||
    kind === "audio" ||
    kind === "mindmap" ||
    kind === "pdf" ||
    kind === "presentation" ||
    kind === "document" ||
    kind === "media"
  );
}
