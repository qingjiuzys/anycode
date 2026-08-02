export interface AppleMediaCapabilities {
  stt: boolean;
  ocr: boolean;
  tts: boolean;
  notify: boolean;
  keychain: boolean;
  pasteboard: boolean;
  platform: string;
  helper_path?: string | null;
  speech_authorized?: boolean | null;
  microphone_authorized?: boolean | null;
}

let tauriAvailable: boolean | null = null;
let cachedCaps: AppleMediaCapabilities | null = null;

function hasTauriInternals(): boolean {
  if (typeof window === "undefined") return false;
  return "__TAURI_INTERNALS__" in window || "__TAURI__" in window;
}

export function isTauriDesktop(): boolean {
  if (tauriAvailable !== null) return tauriAvailable;
  if (hasTauriInternals()) {
    tauriAvailable = true;
    return true;
  }
  // Embedded desktop serves the SPA from loopback; server injects `dw-tauri` on index.html.
  if (
    typeof document !== "undefined" &&
    document.documentElement.classList.contains("dw-tauri")
  ) {
    tauriAvailable = true;
    return true;
  }
  tauriAvailable = false;
  return false;
}

const APPLE_MEDIA_TIMEOUT_MS = 90_000;

async function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new Error("apple_media_timeout")), ms);
  });
  try {
    return await Promise.race([promise, timeout]);
  } finally {
    if (timer !== undefined) clearTimeout(timer);
  }
}
async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const mod = await import("@tauri-apps/api/core");
  const promise = mod.invoke<T>(cmd, args);
  if (cmd === "apple_media_capabilities") {
    return promise;
  }
  return withTimeout(promise, APPLE_MEDIA_TIMEOUT_MS);
}

export async function getAppleMediaCapabilities(): Promise<AppleMediaCapabilities | null> {
  if (!isTauriDesktop()) return null;
  if (cachedCaps) return cachedCaps;
  try {
    cachedCaps = await invokeTauri<AppleMediaCapabilities>("apple_media_capabilities");
    return cachedCaps;
  } catch {
    return null;
  }
}

export async function appleTranscribeAudio(
  blob: Blob,
  locale = "zh-CN",
): Promise<{ ok: true; text: string } | { ok: false; error: string }> {
  if (!isTauriDesktop()) {
    return { ok: false, error: "not_desktop" };
  }
  try {
    const buf = await blob.arrayBuffer();
    const bytes = new Uint8Array(buf);
    let binary = "";
    for (let i = 0; i < bytes.length; i += 1) {
      binary += String.fromCharCode(bytes[i]!);
    }
    const text = await invokeTauri<string>("apple_media_transcribe", {
      audioBase64: btoa(binary),
      mimeType: blob.type || "audio/wav",
      locale,
    });
    return { ok: true, text };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

export async function appleOcrImage(
  mimeType: string,
  dataBase64: string,
  languages?: string[],
): Promise<{ ok: true; text: string } | { ok: false; error: string }> {
  if (!isTauriDesktop()) {
    return { ok: false, error: "not_desktop" };
  }
  try {
    const text = await invokeTauri<string>("apple_media_ocr_image", {
      imageBase64: dataBase64,
      mimeType,
      languages,
    });
    return { ok: true, text };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

export function isAppleSpeechProvider(provider?: string | null): boolean {
  return provider?.trim().toLowerCase() === "apple_speech";
}

export function isAppleTtsProvider(provider?: string | null): boolean {
  return provider?.trim().toLowerCase() === "apple_tts";
}

/** Prefer on-device Apple Speech in the desktop app when the helper is available. */
export function shouldUseNativeAppleSpeech(
  sttProvider: string | null | undefined,
  appleMediaStt?: boolean | null,
): boolean {
  if (!isTauriDesktop()) return false;
  if (isAppleSpeechProvider(sttProvider)) return true;
  return appleMediaStt === true;
}

/** Native folder picker (Desktop only). Returns absolute path or null if cancelled / unavailable. */
export async function pickDirectory(): Promise<string | null> {
  if (!isTauriDesktop()) return null;
  try {
    const path = await invokeTauri<string | null>("pick_directory");
    if (!path || !path.trim()) return null;
    return path.trim();
  } catch {
    return null;
  }
}

export type ApplePasteboardItem = {
  kind: string;
  mime_type?: string | null;
  text?: string | null;
  data_base64?: string | null;
};

/** Read macOS pasteboard via anycode-apple-media (images, text, file URLs). */
export async function readApplePasteboard(): Promise<ApplePasteboardItem[]> {
  if (!isTauriDesktop()) return [];
  try {
    return await invokeTauri<ApplePasteboardItem[]>("apple_media_read_pasteboard");
  } catch {
    return [];
  }
}

/** Reset cached desktop detection (tests). */
export function resetDesktopShellCache(): void {
  tauriAvailable = null;
  cachedCaps = null;
}
