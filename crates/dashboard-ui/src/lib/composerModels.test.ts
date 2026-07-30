import { describe, expect, it } from "vitest";
import {
  findGlobalDefaultChatId,
  inferAutoFromRegistry,
  listChatModels,
  modelLabel,
  modelLikelySupportsVision,
  chatModelSupportsVision,
  modelSubtitle,
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

  it("sorts cloud auto before other cloud and local models", () => {
    const items = [
      chatItem({ id: "local", provider: "openai", model: "gpt-4" }),
      chatItem({ id: "cloud-agnes", provider: "anycode_cloud", model: "agnes-chat", source: "cloud" }),
      chatItem({ id: "cloud-auto", provider: "anycode_cloud", model: "auto", source: "cloud" }),
    ];
    const options = listChatModels(items);
    expect(options.map((o) => o.id)).toEqual(["cloud-auto", "cloud-agnes", "local"]);
  });

  it("dedupes chat models by provider/model", () => {
    const items = [
      chatItem({ id: "a", provider: "google", model: "gemini-2.5-flash" }),
      chatItem({ id: "b", display_name: "Gemini dup", provider: "google", model: "gemini-2.5-flash" }),
    ];
    expect(listChatModels(items)).toHaveLength(1);
  });

  it("labels models without display_name", () => {
    expect(modelLabel(chatItem({ display_name: null }))).toBe("openai/gpt-4");
  });

  it("shows managed registry id in subtitle", () => {
    expect(
      modelSubtitle(
        chatItem({
          id: "managed-minicpm5-1b",
          display_name: "MiniCPM5-1B (SGLang · native tools)",
          provider: "sglang",
          model: "minicpm5-1b",
          source: "managed_local_runtime",
        }),
      ),
    ).toBe("managed-minicpm5-1b · sglang/minicpm5-1b");
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

  it("detects likely vision models by name", () => {
    expect(modelLikelySupportsVision("gpt-4o")).toBe(true);
    expect(modelLikelySupportsVision("kimi-k2.5")).toBe(true);
    expect(modelLikelySupportsVision("text-embedding-3-small")).toBe(false);
  });

  it("allows attachments when active chat model likely supports vision", () => {
    const registry: ModelsRegistryView = {
      config_present: true,
      active: { chat: "kimi" },
      model_fallback: {},
      items: [chatItem({ id: "kimi", provider: "moonshot", model: "kimi-k2.5" })],
    };
    expect(chatModelSupportsVision(registry)).toBe(true);
  });
});
