/** Build and open the anyCode desktop deep link after device authorization. */

export function anycodeDeepLinkForCode(code: string): string {
  return `anycode://link?code=${encodeURIComponent(code)}`;
}

/**
 * Prefer `redirect_uri` from the login URL when it is an anycode:// link;
 * otherwise fall back to `anycode://link?code=…`.
 */
export function resolveDeviceRedirectUri(
  deviceCode: string,
  redirectUri: string | null,
): string {
  const trimmed = redirectUri?.trim() ?? "";
  if (trimmed.toLowerCase().startsWith("anycode://")) {
    try {
      const url = new URL(trimmed);
      if (!url.searchParams.get("code")) {
        url.searchParams.set("code", deviceCode);
      }
      return url.toString();
    } catch {
      /* fall through */
    }
  }
  return anycodeDeepLinkForCode(deviceCode);
}

export type OpenDeepLinkOptions = {
  /**
   * When true (default), also `location.assign` the custom protocol.
   * Set false when the portal should keep navigating (e.g. to /console).
   */
  replacePage?: boolean;
};

/**
 * Navigate to the custom protocol so the OS can show “Open anyCode?”.
 * Best-effort: browsers often block this outside a direct click handler,
 * so callers should also render a visible `<a href={…}>` fallback.
 */
export function openAnycodeDeepLink(
  href: string,
  options: OpenDeepLinkOptions = {},
): void {
  const replacePage = options.replacePage !== false;
  try {
    const iframe = document.createElement("iframe");
    iframe.style.display = "none";
    iframe.setAttribute("aria-hidden", "true");
    iframe.src = href;
    document.body.appendChild(iframe);
    window.setTimeout(() => iframe.remove(), 2000);
  } catch {
    /* ignore */
  }
  if (!replacePage) return;
  try {
    window.location.assign(href);
  } catch {
    const anchor = document.createElement("a");
    anchor.href = href;
    anchor.rel = "noopener";
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
  }
}
