import { describe, expect, it, beforeEach } from "vitest";
import { isAppleSpeechProvider, isTauriDesktop, resetDesktopShellCache } from "./desktopShell";

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
});
