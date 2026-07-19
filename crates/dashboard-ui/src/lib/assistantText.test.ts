import { describe, expect, it } from "vitest";
import {
  sanitizeAssistantDisplay,
  stripTrailingChineseTail,
  stripTrailingEnglishTail,
} from "./assistantText";

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

describe("stripTrailingChineseTail", () => {
  it("removes Chinese tail after English in en locale", () => {
    const text =
      "Here is the analysis.\n\n关键发现：项目状态良好。\n\n总结：可以继续推进。";
    expect(stripTrailingChineseTail(text, "en")).toBe("Here is the analysis.");
  });

  it("does not strip in zh locale", () => {
    const text = "English\n\n关键发现";
    expect(stripTrailingChineseTail(text, "zh")).toBe(text);
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

  it("strips Key findings scaffold before Chinese body in zh locale", () => {
    const text =
      "**Key findings:**\n- The project is at v0.2.4\n- There is a closure plan\n\n## 项目当前诊断\n\n版本 v0.2.4。";
    expect(sanitizeAssistantDisplay(text, "zh")).toContain("项目当前诊断");
    expect(sanitizeAssistantDisplay(text, "zh")).not.toContain("Key findings");
  });

  it("strips mixed English prefix in single paragraph", () => {
    const text =
      "Let me provide a clear analysis. 结合以上信息，我对 anyCode 项目给出分析。";
    expect(sanitizeAssistantDisplay(text, "zh")).toBe(
      "结合以上信息，我对 anyCode 项目给出分析。",
    );
  });

  it("strips Now let me scaffold in zh locale", () => {
    const text =
      "Now let me look at the working tree changes (unstaged modifications compared to HEAD).";
    expect(sanitizeAssistantDisplay(text, "zh")).toBe("");
  });

  it("strips Now let me before Chinese body", () => {
    const text =
      "Now let me also check the core changes.\n\n## 版本审计\n\n当前为 v0.2.4。";
    expect(sanitizeAssistantDisplay(text, "zh")).toContain("版本审计");
    expect(sanitizeAssistantDisplay(text, "zh")).not.toContain("Now let me");
  });

  it("keeps substantive English-only replies in zh locale", () => {
    const text = "English only response without Chinese.";
    expect(sanitizeAssistantDisplay(text, "zh")).toBe(text);
  });

  it("does not sanitize in en locale for English meta", () => {
    const text = 'The user wants me to send "hello".';
    expect(sanitizeAssistantDisplay(text, "en")).toBe(text);
  });

  it("strips leading Chinese scaffold before English reply in en locale", () => {
    const text =
      "关键发现：项目处于 v0.2.4。\n\n让我总结一下。\n\nThe project is at v0.2.4 with a clear roadmap.";
    expect(sanitizeAssistantDisplay(text, "en")).toBe(
      "The project is at v0.2.4 with a clear roadmap.",
    );
  });

  it("strips lone decorative *** / --- lines", () => {
    const text = "**HTML**\n***\nbody copy\n---\nmore";
    expect(sanitizeAssistantDisplay(text, "zh")).toBe("**HTML**\n\nbody copy\n\nmore");
  });
});
