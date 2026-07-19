import { describe, expect, it } from "vitest";
import {
  filterSkillsByCategory,
  normalizeSkillCategory,
  skillDisplayDescription,
  skillDisplayName,
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

  it("uses built-in Chinese description when description_zh is missing", () => {
    const text = skillDisplayDescription(
      {
        id: "anycode-release",
        description: "Build the anycode release binary after code changes.",
      },
      "zh",
    );
    expect(text).toBe("在 anyCode 仓库改代码后构建发布二进制。");
  });

  it("ignores slug-like name_zh and falls back to catalog map", () => {
    expect(
      skillDisplayName(
        { id: "report-to-csv", name: "report-to-csv", name_zh: "report-to-csv" },
        "zh",
      ),
    ).toBe("报表转 CSV");
  });

  it("uses case-insensitive id for catalog map", () => {
    expect(
      skillDisplayName({ id: "Report-To-Csv", name: "Report-To-Csv" }, "zh"),
    ).toBe("报表转 CSV");
  });

  it("prefers Chinese name in zh locale", () => {
    expect(
      skillDisplayName({ id: "report-to-csv", name: "report-to-csv" }, "zh"),
    ).toBe("报表转 CSV");
    expect(
      skillDisplayName({ id: "demo", name: "demo", name_zh: "演示技能" }, "zh"),
    ).toBe("演示技能");
  });

  it("keeps English name in en locale", () => {
    expect(
      skillDisplayName({ id: "report-to-csv", name: "report-to-csv" }, "en"),
    ).toBe("report-to-csv");
  });

  it("filters by category and search", () => {
    const rows = [
      { id: "a", category: "business", description: "日报" },
      { id: "b", category: "data", description: "csv" },
    ];
    expect(filterSkillsByCategory(rows, "data")).toHaveLength(1);
    expect(skillMatchesSearch(rows[0], "日报")).toBe(true);
    expect(skillMatchesSearch({ id: "cn-daily-brief", description: "x" }, "中文日报")).toBe(true);
    expect(skillMatchesSearch(rows[1], "日报")).toBe(false);
  });
});
