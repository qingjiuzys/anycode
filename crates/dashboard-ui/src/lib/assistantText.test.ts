import { describe, expect, it } from "vitest";
import { sanitizeAssistantDisplay, stripTrailingEnglishTail } from "./assistantText";

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

describe("sanitizeAssistantDisplay", () => {
  it("hides pure English meta narration in zh locale", () => {
    const text = 'The user wants me to send a WeChat message saying "你好".';
    expect(sanitizeAssistantDisplay(text, "zh")).toBe("");
  });

  it("strips leading English meta before Chinese reply", () => {
    const text =
      'The user is asking me to resend.\n\nLet me check the context.\n\n已发送"你好"到你的微信。';
    expect(sanitizeAssistantDisplay(text, "zh")).toBe('已发送"你好"到你的微信。');
  });

  it("strips English tail after Chinese reply", () => {
    const text =
      '已发送"你好"到你的微信。\n\nThe WeChat message "你好" was sent successfully.';
    expect(sanitizeAssistantDisplay(text, "zh")).toBe('已发送"你好"到你的微信。');
  });

  it("keeps substantive English-only replies in zh locale", () => {
    const text = "English only response without Chinese.";
    expect(sanitizeAssistantDisplay(text, "zh")).toBe(text);
  });

  it("does not sanitize in en locale", () => {
    const text = 'The user wants me to send "hello".';
    expect(sanitizeAssistantDisplay(text, "en")).toBe(text);
  });
});
