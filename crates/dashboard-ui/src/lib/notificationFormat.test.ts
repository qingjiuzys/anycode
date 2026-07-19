import { describe, expect, it } from "vitest";
import {
  formatNotificationChannelLabel,
  formatNotificationEventLabel,
} from "./notificationFormat";

function t(key: string): string {
  const zh: Record<string, string> = {
    "settings.notificationEvents.gate_failed.name": "门禁失败",
    "settings.notificationChannels.local_log": "本地日志",
    "eventTypes.other": "其他事件",
  };
  return zh[key] ?? key;
}

describe("notificationFormat", () => {
  it("localizes preset notification events", () => {
    expect(formatNotificationEventLabel("gate_failed", t)).toBe("门禁失败");
  });

  it("localizes notification channels", () => {
    expect(formatNotificationChannelLabel("local_log", t)).toBe("本地日志");
  });

  it("falls back to raw channel id when unknown", () => {
    expect(formatNotificationChannelLabel("webhook", t)).toBe("webhook");
  });
});
