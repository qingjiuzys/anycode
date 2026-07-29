export const GRILL_COMPOSER_MODE = "grill" as const;

const STORAGE_PREFIX = "anycode-grill-mode:";

export function grillSlashCommand(locale: "zh" | "en"): string {
  return locale === "zh" ? "拷问" : "grill-me";
}

export function isGrillSlashToken(token: string): boolean {
  const t = token.trim().toLowerCase();
  return t === "拷问" || t === "grill-me" || t === "grill";
}

export function loadGrillMode(sessionKey: string | undefined): boolean {
  if (!sessionKey || typeof sessionStorage === "undefined") return false;
  return sessionStorage.getItem(`${STORAGE_PREFIX}${sessionKey}`) === "1";
}

export function saveGrillMode(sessionKey: string | undefined, on: boolean): void {
  if (!sessionKey || typeof sessionStorage === "undefined") return;
  const key = `${STORAGE_PREFIX}${sessionKey}`;
  if (on) sessionStorage.setItem(key, "1");
  else sessionStorage.removeItem(key);
}

/** User phrases that end grill mode after send. */
export function shouldExitGrillMode(text: string): boolean {
  const s = text.trim();
  if (!s) return false;
  return (
    /可以动手了/.test(s) ||
    /开始实现/.test(s) ||
    /开始写代码/.test(s) ||
    /动手吧/.test(s) ||
    /退出拷问/.test(s) ||
    /退出\s*grill/i.test(s) ||
    /\bready to implement\b/i.test(s) ||
    /\bstart implementing\b/i.test(s) ||
    /\bgo ahead\b/i.test(s)
  );
}

export function composerModeForSend(grillActive: boolean): string | undefined {
  return grillActive ? GRILL_COMPOSER_MODE : undefined;
}
