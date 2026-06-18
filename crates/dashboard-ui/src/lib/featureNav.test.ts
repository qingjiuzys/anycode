import { describe, expect, it } from "vitest";
import { featureNavByPath, FEATURE_NAV, navCount } from "@/lib/featureNav";

describe("featureNav", () => {
  it("resolves settings path", () => {
    expect(featureNavByPath("/settings")?.id).toBe("settings");
    expect(featureNavByPath("/settings?section=prefs")?.id).toBe("settings");
  });

  it("resolves nested project paths", () => {
    expect(featureNavByPath("/projects/abc")?.id).toBe("projects");
  });

  it("counts overview badges", () => {
    const ov = {
      projects_count: 2,
      sessions_total: 10,
      artifacts_count: 5,
      skills_count: 3,
    };
    expect(navCount("projects", ov)).toBe(2);
    expect(navCount(null, ov)).toBeNull();
  });

  it("includes core feature entries", () => {
    const ids = FEATURE_NAV.map((item) => item.id);
    expect(ids).toContain("settings");
    expect(ids).toContain("projects");
    expect(ids).not.toContain("conversations");
  });
});
