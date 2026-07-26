import { describe, expect, it } from "vitest";
import { pluginDisplayDescription, pluginDisplayName } from "./pluginCatalog";

const t = (key: string) => key;

describe("pluginCatalog", () => {
  it("falls back to manifest name for unknown plugins", () => {
    expect(pluginDisplayName("custom-plugin", "My Plugin", t)).toBe("My Plugin");
    expect(pluginDisplayDescription("custom-plugin", t)).toBeNull();
  });
});
