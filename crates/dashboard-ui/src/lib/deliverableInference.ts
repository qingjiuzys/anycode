import type { DeliverableCardProps } from "@/components/deliverables/DeliverableCard";
import { kindForPath } from "@/lib/artifactKind";
import {
  markerShouldInline,
  parseArtifactMarkers,
  type ParsedArtifactMarker,
} from "@/lib/artifactMarker";
import {
  isPrimaryDeliverableKind,
  isProcessArtifactPath,
} from "@/lib/deliverablePath";
import { toolStepFailed, type ToolStep } from "@/lib/transcriptGrouping";
import { extractToolCommand } from "@/lib/toolStepLabel";
import type { TranscriptBlock } from "@/api/types";

/** Paths inferred from prose / tool writes — prefer final formats, not every .md note. */
const INLINE_FILE_RE = /\.(html|htm|pdf|pptx?|xlsx?|csv|json|md|markdown)$/i;
const WRITE_MENTION_RE =
  /(?:已写入|written to|wrote to|created|保存为|输出到|写入)\s*[`'"]?([^\s`'"，。；;]+)/gi;
const BACKTICK_FILE_RE =
  /[`'"]([^\s`'"，。；;]+\.(?:md|markdown|html|pdf|pptx?|xlsx?|csv|json))[`'"]/gi;
const MINDMAP_FILE_RE =
  /(?:^|[\s(,（「『"'`])((?:[\w./\\-]*mindmap[\w.-]*|mind-map[\w.-]*)\.(?:md|markdown))(?=$|[\s),。，；;」』"'`])/gi;

function normalizeMentionedPath(raw: string): string {
  return raw.trim().replace(/^[`'"]+|[`'"]+$/g, "");
}

function isPlausibleDeliverablePath(path: string): boolean {
  if (!path || path.length > 512 || /\s/.test(path)) return false;
  return INLINE_FILE_RE.test(path);
}

function markerFromPath(path: string, title?: string): ParsedArtifactMarker | null {
  if (!isPlausibleDeliverablePath(path)) return null;
  if (isProcessArtifactPath(path)) return null;
  const kind = kindForPath(path);
  // Auto-inferred cards: only primary deliverables (mindmap/html/office/…), not every .md.
  if (!isPrimaryDeliverableKind(kind)) return null;
  const marker: ParsedArtifactMarker = { path, kind, inline: true };
  if (title) marker.title = title;
  return markerShouldInline(marker) ? marker : null;
}

/** Infer inline deliverable paths mentioned in assistant prose (no ANYCODE_ARTIFACT line). */
export function parseDeliverablePathMentions(text: string): ParsedArtifactMarker[] {
  const paths = new Set<string>();
  const add = (raw: string | undefined) => {
    if (!raw) return;
    const path = normalizeMentionedPath(raw);
    if (isPlausibleDeliverablePath(path)) paths.add(path);
  };

  for (const match of text.matchAll(WRITE_MENTION_RE)) add(match[1]);
  for (const match of text.matchAll(BACKTICK_FILE_RE)) add(match[1]);
  for (const match of text.matchAll(MINDMAP_FILE_RE)) add(match[1]);

  const lower = text.toLowerCase();
  const mindmapContext =
    lower.includes("mindmap") ||
    lower.includes("mind-map") ||
    text.includes("思维导图") ||
    text.includes("大纲");

  const out: ParsedArtifactMarker[] = [];
  for (const path of paths) {
    const pathLower = path.toLowerCase();
    const forceMindmap =
      mindmapContext || pathLower.includes("mindmap") || pathLower.includes("mind-map");
    let marker = forceMindmap
      ? (() => {
          if (!isPlausibleDeliverablePath(path) || isProcessArtifactPath(path)) return null;
          const m: ParsedArtifactMarker = { path, kind: "mindmap", inline: true };
          return markerShouldInline(m) ? m : null;
        })()
      : markerFromPath(path);
    if (!marker) continue;
    if (markerShouldInline(marker)) out.push(marker);
  }
  return out;
}

function toolName(step: ToolStep): string {
  const raw =
    (step.result?.meta?.name as string | undefined) ??
    (step.call?.meta?.name as string | undefined) ??
    step.result?.title ??
    step.call?.title ??
    "";
  return raw.replace(/\s+(started|finished|failed)$/i, "").trim();
}

function pathFromJsonBody(body: string): string | null {
  const trimmed = body.trim();
  if (!trimmed.startsWith("{")) return null;
  try {
    const parsed = JSON.parse(trimmed) as Record<string, unknown>;
    if (typeof parsed.path === "string" && parsed.path.trim()) {
      return parsed.path.trim();
    }
  } catch {
    const match = trimmed.match(/"path"\s*:\s*"((?:\\.|[^"\\])*)"/);
    if (match?.[1]) return match[1].replace(/\\"/g, '"').replace(/\\\\/g, "\\");
  }
  return null;
}

/** Extract a written file path from a completed FileWrite / Edit tool step. */
export function pathFromWriteToolStep(step: ToolStep): string | null {
  if (toolStepFailed(step) || !step.result) return null;
  const name = toolName(step);
  if (name !== "FileWrite" && name !== "Edit") return null;

  const meta = (step.result.meta ?? {}) as Record<string, unknown>;
  if (typeof meta.path === "string" && meta.path.trim()) {
    return meta.path.trim();
  }

  const fromBody = pathFromJsonBody(step.result.body ?? "");
  if (fromBody) return fromBody;

  const fromCall = extractToolCommand(step.call);
  return fromCall.trim() || null;
}

type ReplyItem =
  | { kind: "block"; block: TranscriptBlock }
  | {
      kind: "tool_cluster";
      id: string;
      steps: ToolStep[];
      processMessageCount: number;
      processSnippets: string[];
    };

function deliverablePropsFromPath(
  marker: ParsedArtifactMarker,
  projectId?: string,
): DeliverableCardProps {
  return {
    path: marker.path,
    title: marker.title,
    kind: marker.kind,
    mime: marker.mime,
    projectId,
    previewPath: marker.previewPath,
    bytes: marker.bytes,
  };
}

function basename(path: string): string {
  return path.split(/[/\\]/).pop()?.toLowerCase() ?? path.toLowerCase();
}

function familyKey(marker: ParsedArtifactMarker): string {
  const kind = marker.kind ?? kindForPath(marker.path);
  const base = basename(marker.path);
  if (kind === "mindmap") return "mindmap";
  if (kind === "report") return "report";
  if (kind === "spreadsheet") return "spreadsheet";
  if (kind === "presentation") return "presentation";
  if (kind === "document") {
    if (/\.(docx?|xlsx?|csv)$/.test(base)) {
      return base.replace(/-(?:deep|depth|complex|draft|v\d+)\./, ".");
    }
    return `document:${base}`;
  }
  return `${kind}:${base}`;
}

/** When the assistant did not name a file, keep the last write per deliverable family. */
function pickLastToolMarkers(markers: ParsedArtifactMarker[]): ParsedArtifactMarker[] {
  const byFamily = new Map<string, ParsedArtifactMarker>();
  for (const marker of markers) {
    byFamily.set(familyKey(marker), marker);
  }
  return [...byFamily.values()];
}

/** Prefer files the final assistant explicitly names over earlier trial writes. */
function filterToolMarkersForAssistant(
  toolMarkers: ParsedArtifactMarker[],
  assistantBody: string,
): ParsedArtifactMarker[] {
  if (toolMarkers.length === 0) return [];

  const explicit = [
    ...parseArtifactMarkers(assistantBody).filter(markerShouldInline),
    ...parseDeliverablePathMentions(assistantBody),
  ];
  if (explicit.length > 0) {
    const allowed = new Set(explicit.map((marker) => basename(marker.path)));
    const matched = toolMarkers.filter((marker) => allowed.has(basename(marker.path)));
    if (matched.length > 0) return matched;
  }

  return pickLastToolMarkers(toolMarkers);
}

function deliverablePathFromBlock(block: TranscriptBlock): string | null {
  const meta = block.meta ?? {};
  const path = typeof meta.path === "string" ? meta.path.trim() : "";
  return path || null;
}

/** Collect inline deliverables from markers, prose mentions, tool writes, and transcript blocks. */
export function collectInlineDeliverables(
  replyItems: ReplyItem[],
  projectId?: string,
): Map<string, DeliverableCardProps[]> {
  const existingPaths = new Set<string>();
  const existingBasenames = new Set<string>();
  for (const item of replyItems) {
    if (item.kind !== "block" || item.block.block_type !== "deliverable") continue;
    const path = deliverablePathFromBlock(item.block);
    if (!path) continue;
    existingPaths.add(path);
    existingBasenames.add(path.split(/[/\\]/).pop()?.toLowerCase() ?? path.toLowerCase());
  }

  const byBlockId = new Map<string, DeliverableCardProps[]>();
  const push = (blockId: string, marker: ParsedArtifactMarker) => {
    if (isProcessArtifactPath(marker.path)) return;
    const base = marker.path.split(/[/\\]/).pop()?.toLowerCase() ?? marker.path.toLowerCase();
    if (existingBasenames.has(base)) return;
    existingBasenames.add(base);
    existingPaths.add(marker.path);
    const list = byBlockId.get(blockId) ?? [];
    list.push(deliverablePropsFromPath(marker, projectId));
    byBlockId.set(blockId, list);
  };

  let lastAssistantId: string | null = null;
  let lastAssistantBody = "";
  const toolMarkers: ParsedArtifactMarker[] = [];

  for (const item of replyItems) {
    if (item.kind === "tool_cluster") {
      for (const step of item.steps) {
        const path = pathFromWriteToolStep(step);
        const marker = path ? markerFromPath(path) : null;
        if (marker) toolMarkers.push(marker);
      }
    } else if (item.block.block_type === "assistant_message") {
      lastAssistantId = item.block.id;
      lastAssistantBody = item.block.body ?? "";
    }
  }

  const attachBlockId = lastAssistantId;
  if (attachBlockId) {
    for (const marker of filterToolMarkersForAssistant(toolMarkers, lastAssistantBody)) {
      push(attachBlockId, marker);
    }
  }

  for (const item of replyItems) {
    if (item.kind !== "block" || item.block.block_type !== "assistant_message") continue;
    for (const marker of parseArtifactMarkers(item.block.body).filter(markerShouldInline)) {
      push(item.block.id, marker);
    }
    for (const marker of parseDeliverablePathMentions(item.block.body)) {
      push(item.block.id, marker);
    }
  }

  return byBlockId;
}
