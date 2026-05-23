/** Map backend health check names/messages to i18n keys under healthChecks.* */

export function translateHealthField(
  t: (key: string) => string,
  value: string,
): string {
  const key = `healthChecks.${value}`;
  const translated = t(key);
  if (translated !== key) return translated;

  if (value.startsWith("Root path missing:")) {
    const path = value.slice("Root path missing:".length).trim();
    const prefix = t("healthChecks.rootMissingPrefix");
    if (prefix !== "healthChecks.rootMissingPrefix") {
      return `${prefix}: ${path}`;
    }
  }

  return value;
}
