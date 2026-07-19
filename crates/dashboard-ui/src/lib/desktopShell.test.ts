import { describe, expect, it, beforeEach, vi } from "vitest";
import {
  isAppleSpeechProvider,
  isTauriDesktop,
  resetDesktopShellCache,
  shouldUseNativeAppleSpeech,
} from "./desktopShell";

describe("desktopShell", () => {
  beforeEach(() => {
    resetDesktopShellCache();
  });

  it("isAppleSpeechProvider detects apple_speech", () => {
    expect(isAppleSpeechProvider("apple_speech")).toBe(true);
    expect(isAppleSpeechProvider("local_whisper")).toBe(false);
  });

  it("isTauriDesktop is false without globals", () => {
    expect(isTauriDesktop()).toBe(false);
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
});
