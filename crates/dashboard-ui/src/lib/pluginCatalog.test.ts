import { describe, expect, it } from "vitest";
import { pluginDisplayDescription, pluginDisplayName } from "./pluginCatalog";

const t = (key: string) => {
  const zh: Record<string, string> = {
    "settings.plugins.builtin.channelWechat.name": "微信渠道规则",
    "settings.plugins.builtin.channelWechat.description": "通过微信桥接对话时的行为约束。",
  };
  return zh[key] ?? key;
};

describe("pluginCatalog", () => {
  it("localizes built-in plugin name and description", () => {
    expect(pluginDisplayName("channel-wechat", "WeChat Channel Rules", t)).toBe("微信渠道规则");
    expect(pluginDisplayDescription("channel-wechat", t)).toBe("通过微信桥接对话时的行为约束。");
  });

  it("falls back to manifest name for unknown plugins", () => {
    expect(pluginDisplayName("custom-plugin", "My Plugin", t)).toBe("My Plugin");
    expect(pluginDisplayDescription("custom-plugin", t)).toBeNull();
  });
});
