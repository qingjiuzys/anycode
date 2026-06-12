import { describe, expect, it } from "vitest";
import {
  filterSkillsByCategory,
  normalizeSkillCategory,
  skillDisplayDescription,
  skillMatchesSearch,
} from "./skillCatalog";

describe("skillCatalog", () => {
  it("maps legacy office to business", () => {
    expect(normalizeSkillCategory("office")).toBe("business");
    expect(normalizeSkillCategory("dev")).toBe("quality");
  });

  it("prefers Chinese description in zh locale", () => {
    const text = skillDisplayDescription(
      { description: "English", description_zh: "中文" },
      "zh",
    );
    expect(text).toBe("中文");
  });

  it("falls back to English in en locale", () => {
    const text = skillDisplayDescription(
      { description: "English", description_zh: "中文" },
      "en",
    );
    expect(text).toBe("English");
  });

  it("filters by category and search", () => {
    const rows = [
      { id: "a", category: "business", description: "日报" },
      { id: "b", category: "data", description: "csv" },
    ];
    expect(filterSkillsByCategory(rows, "data")).toHaveLength(1);
    expect(skillMatchesSearch(rows[0], "日报")).toBe(true);
    expect(skillMatchesSearch(rows[1], "日报")).toBe(false);
  });
});
