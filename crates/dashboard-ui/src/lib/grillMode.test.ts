import { describe, expect, it } from "vitest";
import {
  composerModeForSend,
  grillSlashCommand,
  isGrillSlashToken,
  shouldExitGrillMode,
} from "./grillMode";

describe("grillMode", () => {
  it("locale slash command names", () => {
    expect(grillSlashCommand("zh")).toBe("拷问");
    expect(grillSlashCommand("en")).toBe("grill-me");
  });

  it("recognizes slash tokens", () => {
    expect(isGrillSlashToken("拷问")).toBe(true);
    expect(isGrillSlashToken("grill-me")).toBe(true);
    expect(isGrillSlashToken("help")).toBe(false);
  });

  it("exit phrases", () => {
    expect(shouldExitGrillMode("好的，可以动手了")).toBe(true);
    expect(shouldExitGrillMode("go ahead")).toBe(true);
    expect(shouldExitGrillMode("继续拷问")).toBe(false);
  });

  it("composer mode payload", () => {
    expect(composerModeForSend(true)).toBe("grill");
    expect(composerModeForSend(false)).toBeUndefined();
  });
});
