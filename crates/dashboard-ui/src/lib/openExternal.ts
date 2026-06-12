/** True when running inside the anyCode desktop (Tauri) shell. */
export function isTauriShell(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Open a URL in the system browser (Tauri) or a new tab (web). */
export async function openExternal(url: string): Promise<void> {
  if (isTauriShell()) {
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(url);
      return;
    } catch (err) {
      console.error("openExternal (tauri shell):", err);
    }
  }

  // Cross-origin window.open often returns null even when the tab opened; never
  // navigate the current dashboard away as a fallback.
  window.open(url, "_blank", "noopener,noreferrer");
}
