import type { CronJobRecord } from "@/api/types";
import type { AutomationTemplate } from "@/lib/automationTemplates";
import { jobDisplayName, templateText } from "@/lib/automationTemplates";

type Locale = "en" | "zh";

/** Codex-style create prompt: explain scheduled tasks, then ask what/when. */
export function buildCreateViaChatPrompt(t: (key: string) => string): string {
  return t("automations.createViaChatPrompt");
}

/** Create prompt seeded from a template suggestion. */
export function buildCreateViaChatPromptFromTemplate(
  t: (key: string) => string,
  tpl: AutomationTemplate,
  locale: Locale,
): string {
  const name = templateText(tpl.name, locale);
  const scheduleLabel = templateText(tpl.schedule_label, locale, tpl.schedule);
  const description = tpl.description ? templateText(tpl.description, locale) : "";
  return t("automations.createViaChatPromptFromTemplate")
    .replaceAll("{name}", name)
    .replaceAll("{schedule}", tpl.schedule || scheduleLabel)
    .replaceAll("{scheduleLabel}", scheduleLabel)
    .replaceAll("{command}", tpl.command ?? "")
    .replaceAll("{description}", description)
    .replaceAll("{toolProfile}", tpl.tool_profile ?? "default");
}

/** Edit prompt with job context for CronUpdate. */
export function buildEditViaChatPrompt(
  t: (key: string) => string,
  job: CronJobRecord,
): string {
  return t("automations.editViaChatPrompt")
    .replaceAll("{id}", job.id)
    .replaceAll("{name}", jobDisplayName(job))
    .replaceAll("{schedule}", job.schedule)
    .replaceAll("{command}", job.command)
    .replaceAll("{enabled}", String(job.enabled ?? true))
    .replaceAll("{timezone}", job.schedule_timezone ?? "local")
    .replaceAll("{failureDestination}", job.failure_destination ?? "log")
    .replaceAll("{toolProfile}", job.tool_profile ?? "default");
}
