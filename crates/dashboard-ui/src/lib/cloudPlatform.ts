/** Canonical hosted platform URLs (override via env / health). */
export const CLOUD_PLATFORM = {
  portalUrl: "https://anycode.work",
  accountApiUrl: "https://anycode.work",
  modelGatewayUrl: "https://anycode.work",
  loginPath: "/login",
} as const;

export function cloudLoginUrl(portalBase: string = CLOUD_PLATFORM.portalUrl): string {
  return `${portalBase.replace(/\/$/, "")}${CLOUD_PLATFORM.loginPath}`;
}
