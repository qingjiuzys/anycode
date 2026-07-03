import { isTauriDesktop } from "@/lib/desktopShell";

type TFn = (key: string) => string;

/** User-facing copy when `/api/health` is unreachable. */
export function apiConnectionMessage(
  t: TFn,
  phase: "loading" | "error",
): { text: string; showLoopbackHint: boolean } {
  if (isTauriDesktop()) {
    if (phase === "loading") {
      return { text: t("home.apiStartingDesktop"), showLoopbackHint: false };
    }
    return { text: t("home.apiErrorDesktop"), showLoopbackHint: false };
  }
  if (phase === "loading") {
    return { text: t("home.apiStarting"), showLoopbackHint: false };
  }
  return { text: t("home.apiError"), showLoopbackHint: true };
}
