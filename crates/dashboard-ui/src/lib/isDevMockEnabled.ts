/** True when mock upgrade / preview banners are allowed (local dev only). */
export function isDevMockEnabled(): boolean {
  return import.meta.env.DEV;
}
