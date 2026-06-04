import type { Locale } from "@/i18n/context";

const DOCS_ORIGIN = "https://qingjiuzys.github.io/anycode";

/** Public docs site home (locale-aware). */
export function docsHomeUrl(locale: Locale): string {
  return locale === "zh" ? `${DOCS_ORIGIN}/zh/` : `${DOCS_ORIGIN}/`;
}

/** In-app Help opens the terminal usage guide. */
export function helpGuideUrl(locale: Locale): string {
  return locale === "zh"
    ? `${DOCS_ORIGIN}/zh/guide/cli`
    : `${DOCS_ORIGIN}/guide/cli`;
}
