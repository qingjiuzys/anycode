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
  // Only the real Tauri WebView exposes these globals. Do **not** treat the
  // server-injected `dw-tauri` CSS class as proof of Desktop — Chrome on the
  // ephemeral loopback URL used to look "native" and break invoke.
  if (hasTauriInternals()) {
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

let harnessSsoBearer: string | null = null;

/** Reset cached desktop detection (tests). */
export function resetDesktopShellCache(): void {
  tauriAvailable = null;
  cachedCaps = null;
  lastHarnessDeviceId = null;
  harnessSsoBearer = null;
}

export const HARNESS_DEVICE_KEYCHAIN_SERVICE = "anycode.harness.device";
const HARNESS_DEVICE_ID_SESSION_KEY = "anycode_harness_device_id";

let lastHarnessDeviceId: string | null = null;

export function isHarnessDeviceKeychainService(service: string | null | undefined): boolean {
  return service === HARNESS_DEVICE_KEYCHAIN_SERVICE;
}

export function isHarnessDeviceAccount(account: string | null | undefined): boolean {
  return (
    typeof account === "string" &&
    /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/.test(account)
  );
}

export function isHarnessDeviceToken(token: string | null | undefined): boolean {
  if (typeof token !== "string") return false;
  const pairingToken = token.trim();
  return pairingToken.length === 43 && /^[A-Za-z0-9_-]+$/.test(pairingToken);
}

function rememberHarnessDeviceId(deviceId: string): void {
  lastHarnessDeviceId = deviceId;
  try {
    sessionStorage.setItem(HARNESS_DEVICE_ID_SESSION_KEY, deviceId);
  } catch {
    /* private mode */
  }
}

export function rememberedHarnessDeviceId(): string | null {
  if (lastHarnessDeviceId) return lastHarnessDeviceId;
  try {
    const stored = sessionStorage.getItem(HARNESS_DEVICE_ID_SESSION_KEY);
    if (stored && isHarnessDeviceAccount(stored)) {
      lastHarnessDeviceId = stored;
      return stored;
    }
  } catch {
    /* private mode */
  }
  return null;
}

/** Persist the one-shot pairing token in the OS keychain. Never writes the token to storage. */
export async function persistHarnessDeviceToken(
  service: string,
  account: string,
  token: string,
): Promise<{ ok: true } | { ok: false; error: string }> {
  if (!isHarnessDeviceKeychainService(service)) {
    return { ok: false, error: "keychain_service_denied" };
  }
  if (!isHarnessDeviceAccount(account)) {
    return { ok: false, error: "device_account_invalid" };
  }
  const pairingToken = token.trim();
  if (!isHarnessDeviceToken(pairingToken)) {
    return { ok: false, error: "device_token_invalid" };
  }
  if (!isTauriDesktop()) {
    return { ok: false, error: "not_desktop" };
  }
  try {
    await invokeTauri<void>("harness_device_keychain_set", { account, token: pairingToken });
    rememberHarnessDeviceId(account);
    return { ok: true };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

/** Memory-only SSO bearer for outbound Desktop pairing. Never written to storage. */
export function setHarnessSsoBearer(token: string | null): void {
  const trimmed = token?.trim() ?? "";
  harnessSsoBearer = trimmed ? trimmed : null;
}

export function hasHarnessSsoBearer(): boolean {
  return Boolean(harnessSsoBearer);
}

export async function desktopPairingOriginConfigured(): Promise<boolean> {
  if (!isTauriDesktop()) return false;
  try {
    const status = await invokeTauri<{ configured?: boolean }>(
      "harness_pairing_origin_configured",
    );
    return status.configured === true;
  } catch {
    return false;
  }
}

export async function desktopPairingPost<T>(
  path: string,
  body: unknown,
): Promise<T> {
  if (!isTauriDesktop()) {
    throw new Error("not_desktop");
  }
  return invokeTauri<T>("harness_pairing_post", {
    path,
    bearer: harnessSsoBearer,
    body,
  });
}

export async function currentHarnessDeviceToken(): Promise<string | null> {
  if (!isTauriDesktop()) return null;
  const account = rememberedHarnessDeviceId();
  if (!account) return null;
  try {
    const token = await invokeTauri<string | null>("harness_device_keychain_get", { account });
    return token && token.trim() ? token : null;
  } catch {
    return null;
  }
}

const WINDOW_DRAG_BLOCK_SELECTOR = [
  "input",
  "textarea",
  "select",
  "button",
  "a",
  "label",
  "summary",
  "[contenteditable='true']",
  "[role='button']",
  "[role='tab']",
  "[role='menuitem']",
  "[role='link']",
  "[role='listbox']",
  "[role='option']",
  "[role='combobox']",
  "[role='textbox']",
  "[role='slider']",
  ".conv-thread-transcript-scroll",
  ".conv-thread-composer",
  ".conv-workbench-panel > div:last-child",
  ".dw-session-sidebar__scroll",
  ".dw-sidebar-quick",
  ".conv-browser-viewport",
  ".conv-workbench-header-icons",
  ".dw-transcript-markdown",
  "pre",
  "code",
  ".conv-git-bar",
  ".cursor-col-resize",
  "[data-no-window-drag]",
  "[data-tauri-drag-region] button",
  "[data-tauri-drag-region] a",
  "[data-tauri-drag-region] input",
  "[data-tauri-drag-region] textarea",
  "[data-tauri-drag-region] select",
  "[data-tauri-drag-region] .dw-session-sidebar__scroll",
  "[data-tauri-drag-region] .dw-sidebar-quick",
].join(",");

export function shouldStartWindowDrag(target: EventTarget | null): boolean {
  if (typeof target !== "object" || target === null || !("closest" in target)) {
    return false;
  }
  const el = target as Element;
  return !el.closest(WINDOW_DRAG_BLOCK_SELECTOR);
}

/** Allow dragging the frameless desktop window from non-interactive chrome / empty areas. */
export function initDesktopWindowDrag(): void {
  if (!isTauriDesktop()) return;
  document.addEventListener(
    "mousedown",
    (event) => {
      if (event.button !== 0) return;
      if (!shouldStartWindowDrag(event.target)) return;
      void import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
        void getCurrentWindow().startDragging();
      });
    },
    true,
  );
}
