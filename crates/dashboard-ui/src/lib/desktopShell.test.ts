import { describe, expect, it, beforeEach, vi } from "vitest";
import {
  HARNESS_DEVICE_KEYCHAIN_SERVICE,
  isAppleSpeechProvider,
  isHarnessDeviceAccount,
  isHarnessDeviceKeychainService,
  isHarnessDeviceToken,
  isTauriDesktop,
  persistHarnessDeviceToken,
  desktopPairingOriginConfigured,
  desktopPairingPost,
  hasHarnessSsoBearer,
  setHarnessSsoBearer,
  resetDesktopShellCache,
  shouldStartWindowDrag,
  shouldUseNativeAppleSpeech,
} from "./desktopShell";

const invokeCalls: Array<{ cmd: string; args: Record<string, unknown> }> = [];

vi.mock("@tauri-apps/api/core", () => ({
  invoke: async (cmd: string, args?: Record<string, unknown>) => {
    invokeCalls.push({ cmd, args: args ?? {} });
  },
}));

describe("desktopShell", () => {
  beforeEach(() => {
    invokeCalls.length = 0;
    resetDesktopShellCache();
  });

  it("isAppleSpeechProvider detects apple_speech", () => {
    expect(isAppleSpeechProvider("apple_speech")).toBe(true);
    expect(isAppleSpeechProvider("local_whisper")).toBe(false);
  });

  it("isTauriDesktop is false without globals", () => {
    expect(isTauriDesktop()).toBe(false);
  });

  it("does not treat the browser as a configured Desktop pairing origin", async () => {
    expect(await desktopPairingOriginConfigured()).toBe(false);
  });

  it("keeps the 818cloud SSO bearer in memory for Desktop pairing_post", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    resetDesktopShellCache();
    setHarnessSsoBearer("sso-opaque-token");
    expect(hasHarnessSsoBearer()).toBe(true);
    await desktopPairingPost("/api/harness/pairing/challenges", { label: "desktop" });
    expect(invokeCalls).toEqual([
      {
        cmd: "harness_pairing_post",
        args: {
          path: "/api/harness/pairing/challenges",
          bearer: "sso-opaque-token",
          body: { label: "desktop" },
        },
      },
    ]);
    resetDesktopShellCache();
    expect(hasHarnessSsoBearer()).toBe(false);
    vi.unstubAllGlobals();
  });

  it("shouldUseNativeAppleSpeech prefers apple media on desktop", () => {
    expect(shouldUseNativeAppleSpeech("local_whisper", true)).toBe(false);
    expect(shouldUseNativeAppleSpeech("apple_speech", false)).toBe(false);
    expect(shouldUseNativeAppleSpeech("local_whisper", false)).toBe(false);

    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    resetDesktopShellCache();
    expect(shouldUseNativeAppleSpeech("local_whisper", true)).toBe(true);
    expect(shouldUseNativeAppleSpeech("apple_speech", false)).toBe(true);
    expect(shouldUseNativeAppleSpeech("local_whisper", false)).toBe(false);
    vi.unstubAllGlobals();
    resetDesktopShellCache();
  });

  it("restricts harness device keychain to the pairing service and UUID account", () => {
    expect(isHarnessDeviceKeychainService(HARNESS_DEVICE_KEYCHAIN_SERVICE)).toBe(true);
    expect(isHarnessDeviceKeychainService("anycode.memory.e2ee")).toBe(false);
    expect(isHarnessDeviceAccount("11111111-1111-1111-1111-111111111111")).toBe(true);
    expect(isHarnessDeviceAccount("not-a-uuid")).toBe(false);
    expect(isHarnessDeviceToken("x".repeat(43))).toBe(true);
    expect(isHarnessDeviceToken("short")).toBe(false);
  });

  it("writes pairing tokens only through harness_device_keychain_set", async () => {
    const store = new Map<string, string>();
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    vi.stubGlobal("sessionStorage", {
      setItem(key: string, value: string) {
        store.set(key, value);
      },
      getItem(key: string) {
        return store.get(key) ?? null;
      },
    });
    resetDesktopShellCache();
    const account = "11111111-1111-1111-1111-111111111111";
    const token = "B".repeat(43);
    const saved = await persistHarnessDeviceToken(
      HARNESS_DEVICE_KEYCHAIN_SERVICE,
      account,
      token,
    );
    expect(saved).toEqual({ ok: true });
    expect(invokeCalls).toEqual([
      { cmd: "harness_device_keychain_set", args: { account, token } },
    ]);
    expect(store.get("anycode_harness_device_id")).toBe(account);
    expect([...store.values()].join("")).not.toContain(token);
    vi.unstubAllGlobals();
    resetDesktopShellCache();
  });

  it("refuses to persist a pairing token outside Desktop or for another service", async () => {
    const denied = await persistHarnessDeviceToken(
      "anycode.memory.e2ee",
      "11111111-1111-1111-1111-111111111111",
      "x".repeat(43),
    );
    expect(denied).toEqual({ ok: false, error: "keychain_service_denied" });
    const notDesktop = await persistHarnessDeviceToken(
      HARNESS_DEVICE_KEYCHAIN_SERVICE,
      "11111111-1111-1111-1111-111111111111",
      "x".repeat(43),
    );
    expect(notDesktop).toEqual({ ok: false, error: "not_desktop" });
    const badToken = await persistHarnessDeviceToken(
      HARNESS_DEVICE_KEYCHAIN_SERVICE,
      "11111111-1111-1111-1111-111111111111",
      "short",
    );
    expect(badToken).toEqual({ ok: false, error: "device_token_invalid" });
    expect(invokeCalls).toEqual([]);
  });

  it("shouldStartWindowDrag respects closest block match", () => {
    const blocked = { closest: () => blocked } as unknown as Element;
    expect(shouldStartWindowDrag(blocked)).toBe(false);

    const allowed = { closest: () => null } as unknown as Element;
    expect(shouldStartWindowDrag(allowed)).toBe(true);
    expect(shouldStartWindowDrag(null)).toBe(false);
  });
});
