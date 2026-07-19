import type { CronJobRecord } from "@/api/types";

export type AutomationTemplate = {
  id: string;
  icon?: string;
  name: string | { en: string; zh: string };
  schedule_label?: string | { en: string; zh: string };
  description?: string | { en: string; zh: string };
  schedule: string;
  schedule_timezone?: string;
  command: string;
  tool_profile?: string;
};

export function templateText(
  value: string | { en: string; zh: string } | undefined,
  locale: "en" | "zh",
  fallback = "",
): string {
  if (!value) return fallback;
  if (typeof value === "string") return value;
  return locale === "zh" ? value.zh : value.en;
}

export function normalizeTemplate(raw: Record<string, unknown>): AutomationTemplate {
  return {
    id: String(raw.id ?? ""),
    icon: typeof raw.icon === "string" ? raw.icon : undefined,
    name: (raw.name as AutomationTemplate["name"]) ?? String(raw.id ?? ""),
    schedule_label: raw.schedule_label as AutomationTemplate["schedule_label"],
    description: raw.description as AutomationTemplate["description"],
    schedule: String(raw.schedule ?? "0 0 9 * * *"),
    schedule_timezone:
      typeof raw.schedule_timezone === "string" ? raw.schedule_timezone : "local",
    command: String(raw.command ?? ""),
    tool_profile: typeof raw.tool_profile === "string" ? raw.tool_profile : undefined,
  };
}

export function jobDisplayName(job: CronJobRecord): string {
  const name = job.name?.trim();
  if (name) return name;
  const cmd = job.command.trim();
  if (cmd.length <= 48) return cmd;
  return `${cmd.slice(0, 48)}…`;
}
