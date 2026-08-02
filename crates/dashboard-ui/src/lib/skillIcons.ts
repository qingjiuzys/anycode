import { registeredIconNames } from "@/components/Icon";
import { normalizeSkillCategory, type SkillCategory } from "./skillCatalog";

export type SkillIconTone =
  | "violet"
  | "blue"
  | "teal"
  | "amber"
  | "rose"
  | "indigo"
  | "orange"
  | "cyan";

export interface SkillIconMeta {
  icon: string;
  tone: SkillIconTone;
}

/** Known starter / repo skills — distinct icons per id. */
const SKILL_ICON_BY_ID: Record<string, string> = {
  "report-to-csv": "bar_chart",
  "cn-daily-brief": "article",
  "cn-meeting-minutes": "rate_review",
  "cn-weekly-report": "timeline",
  "content-repurpose": "content_copy",
  "internal-comms": "forum",
  "doc-summary": "document_scanner",
  "file-organizer": "folder_open",
  "novel-writer": "menu_book",
  "anycode-ppt": "slideshow",
  "anycode-pdf": "description",
  "anycode-docx": "description",
  "anycode-xlsx": "bar_chart",
  "video-script": "movie",
  "anycode-release": "build",
  "flutter-bootstrap": "construction",
  "flutter-gate-fix": "verified",
  "flutter-prd": "edit",
  "flutter-screen-plan": "view_sidebar",
  "flutter-ui-polish": "palette",
};

const CATEGORY_ICON: Record<SkillCategory, string> = {
  office: "description",
  writing: "edit",
  design: "palette",
  research: "analytics",
  engineering: "code",
  ops: "folder_open",
  other: "extension",
};

const CATEGORY_TONE: Record<SkillCategory, SkillIconTone> = {
  office: "amber",
  writing: "rose",
  design: "violet",
  research: "blue",
  engineering: "teal",
  ops: "orange",
  other: "indigo",
};

const TONE_ORDER: SkillIconTone[] = [
  "violet",
  "blue",
  "teal",
  "amber",
  "rose",
  "indigo",
  "orange",
  "cyan",
];

const FALLBACK_ICONS = [
  "analytics",
  "article",
  "attach_file",
  "build",
  "chat",
  "code",
  "content_copy",
  "dashboard",
  "description",
  "document_scanner",
  "download",
  "edit",
  "forum",
  "history",
  "image",
  "inventory",
  "link",
  "menu_book",
  "mic",
  "movie",
  "notifications",
  "palette",
  "policy",
  "psychology",
  "quiz",
  "radar",
  "rate_review",
  "route",
  "schedule",
  "search",
  "send",
  "settings",
  "slideshow",
  "sync",
  "terminal",
  "timeline",
  "tune",
  "upload",
  "visibility",
] as const;

function hashString(value: string): number {
  let hash = 0;
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash * 31 + value.charCodeAt(i)) >>> 0;
  }
  return hash;
}

function iconFromHeuristics(id: string): string | null {
  const lower = id.toLowerCase();
  if (lower.includes("csv") || lower.includes("table") || lower.includes("chart")) {
    return "bar_chart";
  }
  if (lower.includes("pdf")) return "description";
  if (lower.includes("ppt") || lower.includes("slide")) return "slideshow";
  if (lower.includes("video") || lower.includes("movie")) return "movie";
  if (lower.includes("chat")) return "forum";
  if (lower.includes("report") || lower.includes("brief")) return "article";
  if (lower.includes("summary") || lower.includes("doc")) return "document_scanner";
  if (lower.includes("file") || lower.includes("organ")) return "folder_open";
  if (lower.includes("novel") || lower.includes("writer") || lower.includes("book")) {
    return "menu_book";
  }
  if (lower.includes("meeting") || lower.includes("minutes")) return "rate_review";
  if (lower.includes("weekly") || lower.includes("daily")) return "schedule";
  if (lower.includes("flutter") || lower.includes("ui")) return "dashboard_customize";
  if (lower.includes("code") || lower.includes("dev") || lower.includes("contributor")) {
    return "code";
  }
  if (lower.includes("release") || lower.includes("build") || lower.includes("publish")) {
    return "build";
  }
  return null;
}

function ensureRegisteredIcon(name: string): string {
  return registeredIconNames.has(name) ? name : "extension";
}

function fallbackIcon(id: string, category: SkillCategory): string {
  const heuristic = id ? iconFromHeuristics(id) : null;
  if (heuristic) return ensureRegisteredIcon(heuristic);
  if (id) {
    const picked = FALLBACK_ICONS[hashString(id) % FALLBACK_ICONS.length];
    return ensureRegisteredIcon(picked);
  }
  return ensureRegisteredIcon(CATEGORY_ICON[category]);
}

export function skillIconMeta(skill: { id?: string; category?: string | null }): SkillIconMeta {
  const id = (skill.id ?? "").trim();
  const category = normalizeSkillCategory(skill.category);
  const icon = id && SKILL_ICON_BY_ID[id]
    ? ensureRegisteredIcon(SKILL_ICON_BY_ID[id])
    : fallbackIcon(id, category);
  const tone = id
    ? TONE_ORDER[hashString(id) % TONE_ORDER.length]
    : CATEGORY_TONE[category];
  return { icon, tone };
}

export function skillIconToneClass(tone: SkillIconTone): string {
  return `dw-agents-skill-row__icon--${tone}`;
}
