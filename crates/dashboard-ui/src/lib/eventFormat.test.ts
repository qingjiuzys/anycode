import { describe, expect, it } from "vitest";
import {
  eventTypeI18nKey,
  formatEventTitle,
  formatEventTypeLabel,
  formatExecutionLogLine,
  isImportedSessionTitle,
  localizeLogTitle,
} from "./eventFormat";

describe("eventFormat", () => {
  const t = (key: string) => {
    const map: Record<string, string> = {
      "eventTypes.tool_call_end": "工具调用完成",
      "eventTypes.other": "其他事件",
      "eventTitles.toolStarted": "{tool} 已开始",
      "eventTitles.toolDeniedBare": "工具被拒绝",
      "eventTitles.taskStart": "任务开始",
      "eventTitles.turnEnd": "第 {turn} 轮结束",
      "eventTitles.userPrompt": "用户输入",
    };
    return map[key] ?? key;
  };

  it("maps known event types", () => {
    expect(eventTypeI18nKey("tool_call_end")).toBe("eventTypes.tool_call_end");
    expect(formatEventTypeLabel("tool_call_end", t)).toBe("工具调用完成");
  });

  it("detects imported session titles", () => {
    expect(isImportedSessionTitle("Run · imported task abc")).toBe(true);
    expect(isImportedSessionTitle("My session")).toBe(false);
  });

  it("localizes common backend log titles", () => {
    expect(localizeLogTitle("Bash started", "tool_call_start", t)).toBe("Bash 已开始");
    expect(localizeLogTitle("Turn 3 finished", "turn_end", t)).toBe("第 3 轮结束");
    expect(localizeLogTitle("User prompt", "user_prompt", t)).toBe("用户输入");
    expect(localizeLogTitle("Tool denied", "tool_call_end", t)).toBe("工具被拒绝");
    expect(localizeLogTitle("Task start", "task_start", t)).toBe("任务开始");
  });

  it("formats event titles via localizeLogTitle", () => {
    expect(
      formatEventTitle(
        { title: "Bash started", event_type: "tool_call_start", payload: { name: "Bash" } },
        t,
      ),
    ).toBe("Bash 已开始");
  });

  it("formats execution log lines", () => {
    const formatted = formatExecutionLogLine(
      { event_type: "tool_call_start", title: "Bash started", raw: "[tool_call_start]" },
      t,
    );
    expect(formatted.title).toBe("Bash 已开始");
  });
});
