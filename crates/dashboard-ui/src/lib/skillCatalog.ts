import type { Locale } from "@/i18n/context";

/** Product-facing skill taxonomy (secondary tabs on Agents / Skills). */
export const SKILL_CATEGORIES = [
  "office",
  "writing",
  "design",
  "research",
  "engineering",
  "ops",
  "other",
] as const;

export type SkillCategory = (typeof SKILL_CATEGORIES)[number];

/** Map legacy Anthropic-style / older anyCode categories → current taxonomy. */
const LEGACY_CATEGORY_MAP: Record<string, SkillCategory> = {
  office: "office",
  docs: "office",
  business: "office",
  writing: "writing",
  creative: "design",
  design: "design",
  data: "research",
  research: "research",
  quality: "engineering",
  scaffolding: "engineering",
  cicd: "engineering",
  "library-ref": "engineering",
  verification: "engineering",
  engineering: "engineering",
  dev: "engineering",
  runbook: "ops",
  infra: "ops",
  ops: "ops",
  other: "other",
};

export interface SkillDisplayFields {
  id?: string;
  name?: string;
  name_zh?: string | null;
  description?: string;
  description_zh?: string | null;
  category?: string | null;
}

/** Built-in display names for starter + official market skills (zh UI). */
export const SKILL_NAMES_ZH: Record<string, string> = {
  "cn-daily-brief": "中文日报",
  "cn-meeting-minutes": "会议纪要",
  "cn-weekly-report": "中文周报",
  "content-repurpose": "内容改写",
  "doc-summary": "文档摘要",
  "file-organizer": "文件整理",
  "frontend-design": "前端设计",
  "novel-writer": "小说创作",
  "anycode-ppt": "PPT / 幻灯片",
  "anycode-pdf": "PDF 撰写",
  "anycode-docx": "Word 撰写",
  "anycode-xlsx": "Excel 表格",
  docx: "Word 文档",
  pdf: "PDF 文档",
  pptx: "PPT 演示",
  "report-to-csv": "报表转 CSV",
  "video-script": "视频脚本",
  "webapp-testing": "Web 测试",
  "internal-comms": "内部沟通",
  xlsx: "Excel 表格",
  "anycode-release": "anyCode 发布构建",
};

/** Built-in Chinese descriptions when skill metadata lacks description_zh. */
export const SKILL_DESCRIPTIONS_ZH: Record<string, string> = {
  "anycode-release": "在 anyCode 仓库改代码后构建发布二进制。",
};

export function normalizeSkillCategory(raw?: string | null): SkillCategory {
  const c = (raw ?? "").trim().toLowerCase();
  if ((SKILL_CATEGORIES as readonly string[]).includes(c)) {
    return c as SkillCategory;
  }
  return LEGACY_CATEGORY_MAP[c] ?? "other";
}

export function skillDisplayDescription(
  skill: SkillDisplayFields,
  locale: Locale,
): string {
  const id = (skill.id ?? skill.name ?? "").trim().toLowerCase();
  if (locale === "zh") {
    if (skill.description_zh?.trim()) {
      return skill.description_zh.trim();
    }
    if (id && SKILL_DESCRIPTIONS_ZH[id]) {
      return SKILL_DESCRIPTIONS_ZH[id];
    }
  }
  return (skill.description ?? "").trim();
}

/** True when a value looks like an English skill id slug, not a Chinese display name. */
function isEnglishSkillSlug(value: string, id: string, enName: string): boolean {
  const v = value.trim();
  if (!v) return true;
  if (v === id || v === enName) return true;
  return /^[a-z0-9]+(-[a-z0-9]+)*$/i.test(v) && !/[\u4e00-\u9fff]/.test(v);
}

export function skillDisplayName(
  skill: SkillDisplayFields,
  locale: Locale,
): string {
  const id = (skill.id ?? skill.name ?? "").trim();
  const enName = (skill.name ?? id).trim();
  const idKey = id.toLowerCase();

  if (locale === "zh") {
    const zhFromMeta = skill.name_zh?.trim();
    if (zhFromMeta && !isEnglishSkillSlug(zhFromMeta, id, enName)) {
      return zhFromMeta;
    }
    if (idKey && SKILL_NAMES_ZH[idKey]) {
      return SKILL_NAMES_ZH[idKey];
    }
    if (id && SKILL_NAMES_ZH[id]) {
      return SKILL_NAMES_ZH[id];
    }
  }
  return enName || id;
}

export function skillMatchesSearch(
  skill: SkillDisplayFields & { id?: string },
  query: string,
): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  const fields = [
    skill.id,
    skill.name,
    skill.name_zh,
    skill.description,
    skill.description_zh,
    skill.id ? SKILL_NAMES_ZH[skill.id] : undefined,
  ];
  return fields.some((f) => (f ?? "").toLowerCase().includes(q));
}

export function filterSkillsByCategory<T extends SkillDisplayFields>(
  skills: T[],
  category: SkillCategory | "all",
): T[] {
  if (category === "all") return skills;
  return skills.filter((s) => normalizeSkillCategory(s.category) === category);
}

export function categoriesWithEntries<T extends SkillDisplayFields>(
  skills: T[],
): SkillCategory[] {
  const seen = new Set<SkillCategory>();
  for (const s of skills) {
    seen.add(normalizeSkillCategory(s.category));
  }
  return SKILL_CATEGORIES.filter((c) => seen.has(c));
}

export function groupSkillsByCategory<T extends SkillDisplayFields>(
  skills: T[],
): Array<{ category: SkillCategory; items: T[] }> {
  return SKILL_CATEGORIES.map((cat) => ({
    category: cat,
    items: skills.filter((s) => normalizeSkillCategory(s.category) === cat),
  })).filter((g) => g.items.length > 0);
}
