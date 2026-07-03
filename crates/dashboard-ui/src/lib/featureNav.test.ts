import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";
import {
  featureNavByPath,
  FEATURE_NAV,
  isFeatureNavItemVisible,
  navCount,
} from "@/lib/featureNav";
import { HIDE_REPORTS_KEY } from "@/lib/featureFlags";

function mockLocalStorage() {
  const store = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => {
      store.set(key, value);
    },
    removeItem: (key: string) => {
      store.delete(key);
    },
    clear: () => store.clear(),
  });
  return store;
}

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

  describe("hide reports flag", () => {
    beforeEach(() => {
      mockLocalStorage();
    });
    afterEach(() => {
      vi.unstubAllGlobals();
    });

    it("hides reports when localStorage flag is set", () => {
      const reports = FEATURE_NAV.find((item) => item.id === "reports")!;
      expect(isFeatureNavItemVisible(reports)).toBe(true);
      localStorage.setItem(HIDE_REPORTS_KEY, "1");
      expect(isFeatureNavItemVisible(reports)).toBe(false);
    });
  });
});
