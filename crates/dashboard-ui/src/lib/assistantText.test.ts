import { describe, expect, it } from "vitest";
import { stripTrailingEnglishTail } from "./assistantText";

describe("stripTrailingEnglishTail", () => {
  it("removes English tail after Chinese in zh locale", () => {
    const text =
      "这是中文回答。\n\nNow I have a good understanding of the context. Let me summarize.";
    expect(stripTrailingEnglishTail(text, "zh")).toBe("这是中文回答。");
  });

  it("keeps pure Chinese unchanged", () => {
    const text = "纯中文内容。";
    expect(stripTrailingEnglishTail(text, "zh")).toBe(text);
  });

  it("does not strip English-only messages in zh locale", () => {
    const text = "English only response without Chinese.";
    expect(stripTrailingEnglishTail(text, "zh")).toBe(text);
  });

  it("does not strip in en locale", () => {
    const text = "中文\n\nNow I have more to say.";
    expect(stripTrailingEnglishTail(text, "en")).toBe(text);
  });
});
