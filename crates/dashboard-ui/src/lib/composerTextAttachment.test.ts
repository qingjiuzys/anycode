import { describe, expect, it } from "vitest";
import {
  makePastedTextAttachment,
  PASTE_AS_CARD_MIN_CHARS,
  shouldPasteAsTextCard,
  utf8ByteLength,
} from "./composerTextAttachment";

describe("composerTextAttachment", () => {
  it("treats text over 4000 chars as card paste", () => {
    expect(shouldPasteAsTextCard("x".repeat(PASTE_AS_CARD_MIN_CHARS))).toBe(false);
    expect(shouldPasteAsTextCard("x".repeat(PASTE_AS_CARD_MIN_CHARS + 1))).toBe(true);
  });

  it("names pasted cards uniquely", () => {
    expect(makePastedTextAttachment("a", []).filename).toBe("paste-1.txt");
    expect(makePastedTextAttachment("a", ["paste-1.txt"]).filename).toBe("paste-2.txt");
    expect(makePastedTextAttachment("a", ["paste-1.txt", "paste-2.txt"]).filename).toBe(
      "paste-3.txt",
    );
  });

  it("counts utf-8 bytes for size gate", () => {
    expect(utf8ByteLength("abc")).toBe(3);
    expect(utf8ByteLength("你好")).toBe(6);
  });
});
