/** Read dashboard CSS variables for ECharts theming (updates when skin/theme changes). */
export function chartCssVar(name: string, fallback: string): string {
  if (typeof document === "undefined") {
    return fallback;
  }
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

export function chartPalette() {
  return {
    secondary: chartCssVar("--color-secondary", "#505f76"),
    outline: chartCssVar("--color-outline", "#737686"),
    primary: chartCssVar("--accent", "#6e6bff"),
    accentMuted: chartCssVar("--accent-muted", "rgba(110, 107, 255, 0.18)"),
    success: chartCssVar("--color-success", "#16a34a"),
  };
}
