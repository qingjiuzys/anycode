import { SITE_ORIGIN, SITE_PATHS, siteUrl } from "@anycode/site-urls";

/** Canonical hosted platform URLs (override via env / health). */
export const CLOUD_PLATFORM = {
  portalUrl: SITE_ORIGIN,
  accountApiUrl: SITE_ORIGIN,
  modelGatewayUrl: SITE_ORIGIN,
  loginPath: SITE_PATHS.login,
} as const;

export function cloudLoginUrl(portalBase: string = CLOUD_PLATFORM.portalUrl): string {
  return siteUrl("login", portalBase);
}

export { SITE_ORIGIN, SITE_PATHS, siteUrl, legalUrls, docsUrls, SITE_EMAILS } from "@anycode/site-urls";
