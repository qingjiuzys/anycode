import { describe, expect, it } from "vitest";
import {
  findGlobalDefaultChatId,
  inferAutoFromRegistry,
  listChatModels,
  modelLabel,
} from "@/lib/composerModels";
import type { ConfiguredModel, ModelsRegistryView } from "@/api/types";

const chatItem = (overrides: Partial<ConfiguredModel>): ConfiguredModel => ({
  id: "id",
  provider: "openai",
  model: "gpt-4",
  capabilities: ["chat"],
  enabled: true,
  ...overrides,
});

describe("composerModels", () => {
  it("lists enabled chat-capable models", () => {
    const items = [
      chatItem({ id: "a", display_name: "Alpha", provider: "z.ai", model: "glm-5" }),
      chatItem({ id: "b", enabled: false }),
      chatItem({ id: "c", capabilities: ["embedding"] }),
    ];
    const options = listChatModels(items);
    expect(options).toHaveLength(1);
    expect(options[0]).toMatchObject({ id: "a", label: "Alpha", subtitle: "z.ai/glm-5" });
  });

  it("labels models without display_name", () => {
    expect(modelLabel(chatItem({ display_name: null }))).toBe("openai/gpt-4");
  });

  it("finds global default chat id", () => {
    const registry: ModelsRegistryView = {
      config_present: true,
      active: { chat: "other" },
      model_fallback: {},
      global: { provider: "z.ai", model: "glm-5" },
      items: [
        chatItem({ id: "glm", provider: "z.ai", model: "glm-5" }),
        chatItem({ id: "other", provider: "openai", model: "gpt-4" }),
      ],
    };
    expect(findGlobalDefaultChatId(registry)).toBe("glm");
  });

  it("infers auto when active chat matches global default", () => {
    const registry: ModelsRegistryView = {
      config_present: true,
      active: { chat: "glm" },
      model_fallback: {},
      global: { provider: "z.ai", model: "glm-5" },
      items: [chatItem({ id: "glm", provider: "z.ai", model: "glm-5" })],
    };
    expect(inferAutoFromRegistry(registry)).toBe(true);
  });

  it("infers manual when active chat differs from global default", () => {
    const registry: ModelsRegistryView = {
      config_present: true,
      active: { chat: "other" },
      model_fallback: {},
      global: { provider: "z.ai", model: "glm-5" },
      items: [
        chatItem({ id: "glm", provider: "z.ai", model: "glm-5" }),
        chatItem({ id: "other", provider: "openai", model: "gpt-4" }),
      ],
    };
    expect(inferAutoFromRegistry(registry)).toBe(false);
  });
});
