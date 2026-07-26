/** Built-in plugin ids with localized display keys in settings.plugins.builtin.* */
export const PLUGIN_LABEL_KEYS: Record<string, string> = {};

export function pluginLabelKey(id: string): string | null {
  return PLUGIN_LABEL_KEYS[id.trim()] ?? null;
}

/** Localized plugin title; falls back to manifest name for user plugins. */
export function pluginDisplayName(
  id: string,
  fallbackName: string,
  t: (key: string) => string,
): string {
  const labelKey = pluginLabelKey(id);
  if (!labelKey) return fallbackName;
  const label = t(`settings.plugins.builtin.${labelKey}.name`);
  return label === `settings.plugins.builtin.${labelKey}.name` ? fallbackName : label;
}

/** Localized short description for built-in plugins. */
export function pluginDisplayDescription(
  id: string,
  t: (key: string) => string,
): string | null {
  const labelKey = pluginLabelKey(id);
  if (!labelKey) return null;
  const desc = t(`settings.plugins.builtin.${labelKey}.description`);
  return desc === `settings.plugins.builtin.${labelKey}.description` ? null : desc;
}

export function isBuiltinPlugin(id: string): boolean {
  return pluginLabelKey(id) !== null;
}
