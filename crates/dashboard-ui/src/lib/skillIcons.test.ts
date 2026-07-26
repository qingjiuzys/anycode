import { describe, expect, it } from "vitest";
import { skillIconMeta } from "./skillIcons";

describe("skillIcons", () => {
  it("assigns distinct icons for starter skills", () => {
    const csv = skillIconMeta({ id: "report-to-csv", category: "data" });
    const brief = skillIconMeta({ id: "cn-daily-brief", category: "business" });
    const pptx = skillIconMeta({ id: "office-pptx", category: "business" });
    expect(csv.icon).toBe("bar_chart");
    expect(brief.icon).toBe("article");
    expect(pptx.icon).toBe("slideshow");
    expect(new Set([csv.icon, brief.icon, pptx.icon]).size).toBe(3);
  });

  it("falls back to category icon when id is unknown", () => {
    const meta = skillIconMeta({ id: "unknown-skill-xyz", category: "data" });
    expect(meta.icon).not.toBe("extension");
  });

  it("is stable for the same skill id", () => {
    const a = skillIconMeta({ id: "custom-skill-alpha", category: "other" });
    const b = skillIconMeta({ id: "custom-skill-alpha", category: "other" });
    expect(a).toEqual(b);
  });
});
