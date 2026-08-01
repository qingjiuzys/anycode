import { isTauriDesktop } from "@/lib/desktopShell";

/** Reveal a folder/file in the system file manager (Finder / Explorer). */
export async function revealInFileManager(path: string): Promise<void> {
  const target = path.trim();
  if (!target) return;

  if (isTauriDesktop()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("reveal_in_file_manager", { path: target });
    return;
  }

  throw new Error("not_desktop");
}

/** Open a local file/folder with the OS default application. */
export async function openLocalPath(path: string): Promise<void> {
  const target = path.trim();
  if (!target) return;

  if (isTauriDesktop()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("open_local_path", { path: target });
    return;
  }

  throw new Error("not_desktop");
}

/** Open a URL in the system browser (Tauri) or a new tab (web). */
export async function openExternal(url: string): Promise<void> {
  const target = url.trim();
  if (!target) return;

  if (isTauriDesktop()) {
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(target);
      return;
    } catch (err) {
      console.warn("openExternal (shell plugin):", err);
    }
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("open_external_url", { url: target });
      return;
    } catch (err) {
      console.error("openExternal (invoke):", err);
      throw err;
    }
  }

  // Cross-origin window.open often returns null even when the tab opened; never
  // navigate the current dashboard away as a fallback.
  const opened = window.open(target, "_blank", "noopener,noreferrer");
  if (!opened) {
    throw new Error("popup_blocked");
  }
}
