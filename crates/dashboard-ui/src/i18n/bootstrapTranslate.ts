/** Translate backend bootstrap next_steps strings */

export function translateBootstrapStep(t: (key: string) => string, step: string): string {
  const exact = t(`bootstrapSteps.${step}`);
  if (exact !== `bootstrapSteps.${step}`) return exact;

  const gateMatch = step.match(/^(\d+) required gate\(s\) failed — open Projects to review blocked sessions$/);
  if (gateMatch) {
    return t("bootstrapSteps.gatesFailed").replace("{n}", gateMatch[1]!);
  }

  const runningMatch = step.match(/^(\d+) session\(s\) running — check Conversations for live updates$/);
  if (runningMatch) {
    return t("bootstrapSteps.sessionsRunning").replace("{n}", runningMatch[1]!);
  }

  return step;
}
