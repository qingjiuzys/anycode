/**
 * Canonical public site URLs for anyCode (portal + desktop external links).
 *
 * Change `ORIGIN` (and paths below) here when the public host or legal/docs
 * routes move — dashboard-ui and account-portal both import this module.
 */
export const SITE_ORIGIN = "https://anycode.work";

/** Canonical public GitHub repository. */
export const SITE_GITHUB = "https://github.com/qingjiuzys/anycode";

/** Pathnames on the public portal (always start with `/`). */
export const SITE_PATHS = {
  home: "/",
  login: "/login",
  register: "/register",
  features: "/features",
  product: "/product",
  plans: "/plans",
  docs: "/docs",
  docsHelp: "/docs/help",
  legalUserAgreement: "/legal/user-agreement",
  legalPrivacy: "/legal/privacy",
  legalAlgorithmDisclosure: "/legal/algorithm-disclosure",
  desktopDownloadDmg: "/downloads/anyCode_latest_aarch64.dmg",
  downloads: "/downloads",
  changelog: "/changelog",
  cases: "/cases",
} as const;

export type SitePathKey = keyof typeof SITE_PATHS;

export const SITE_EMAILS = {
  support: "support@anycode.work",
  security: "security@anycode.work",
  privacy: "privacy@anycode.work",
} as const;

/** Join origin + path → absolute https URL. */
export function siteUrl(
  path: SitePathKey | (string & {}),
  origin: string = SITE_ORIGIN,
): string {
  const pathname =
    path in SITE_PATHS ? SITE_PATHS[path as SitePathKey] : String(path);
  const base = origin.replace(/\/$/, "");
  const suffix = pathname.startsWith("/") ? pathname : `/${pathname}`;
  return `${base}${suffix}`;
}

export const legalUrls = {
  userAgreement: (origin?: string) => siteUrl("legalUserAgreement", origin),
  privacy: (origin?: string) => siteUrl("legalPrivacy", origin),
  algorithmDisclosure: (origin?: string) =>
    siteUrl("legalAlgorithmDisclosure", origin),
} as const;

export const docsUrls = {
  home: (origin?: string) => siteUrl("docs", origin),
  help: (origin?: string) => siteUrl("docsHelp", origin),
} as const;
