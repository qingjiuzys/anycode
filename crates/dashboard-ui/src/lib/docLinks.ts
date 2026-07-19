import type { Locale } from "@/i18n/context";
import { SITE_ORIGIN, SITE_PATHS, siteUrl } from "@anycode/site-urls";

/** Public docs site home (locale-aware). */
export function docsHomeUrl(locale: Locale, origin: string = SITE_ORIGIN): string {
  const base = siteUrl("docs", origin).replace(/\/$/, "");
  return locale === "zh" ? `${base}/zh/` : `${base}/`;
}

/** In-app Help opens support contact on the official docs site. */
export function helpGuideUrl(locale: Locale, origin: string = SITE_ORIGIN): string {
  const base = siteUrl("docs", origin).replace(/\/$/, "");
  return locale === "zh" ? `${base}/zh/help` : `${base}/help`;
}

/** Project verification gates & guardrails (Workbench tour). */
export function projectGatesDocsUrl(locale: Locale, origin: string = SITE_ORIGIN): string {
  const base = siteUrl("docs", origin).replace(/\/$/, "");
  return locale === "zh" ? `${base}/zh/guide/workbench` : `${base}/guide/workbench`;
}

export { SITE_ORIGIN, SITE_PATHS };
