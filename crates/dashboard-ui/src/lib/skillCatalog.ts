import type { Locale } from "@/i18n/context";

export const SKILL_CATEGORIES = [
  "library-ref",
  "verification",
  "data",
  "business",
  "scaffolding",
  "quality",
  "cicd",
  "runbook",
  "infra",
  "other",
] as const;

export type SkillCategory = (typeof SKILL_CATEGORIES)[number];

const LEGACY_CATEGORY_MAP: Record<string, SkillCategory> = {
  office: "business",
  docs: "business",
  dev: "quality",
  data: "data",
  other: "other",
};

export interface SkillDisplayFields {
  name?: string;
  description?: string;
  description_zh?: string | null;
  category?: string | null;
}

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
  if (locale === "zh" && skill.description_zh?.trim()) {
    return skill.description_zh.trim();
  }
  return (skill.description ?? "").trim();
}

export function skillDisplayName(skill: SkillDisplayFields, locale: Locale): string {
  void locale;
  return (skill.name ?? "").trim();
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
    skill.description,
    skill.description_zh,
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
