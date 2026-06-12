import { describe, expect, it } from "vitest";
import {
  mergeProjectViewPrefs,
  normalizeProjectViewPrefs,
  shouldMigrateLocalToServer,
} from "./projectViewPrefs";

describe("projectViewPrefs", () => {
  it("clamps session flow limit", () => {
    expect(normalizeProjectViewPrefs({ sessionFlowLimit: 99 }).sessionFlowLimit).toBe(20);
    expect(normalizeProjectViewPrefs({ sessionFlowLimit: 1 }).sessionFlowLimit).toBe(3);
  });

  it("merges server over local when server has data", () => {
    const merged = mergeProjectViewPrefs(
      { sessionFlowLimit: 12, acceptancePresetIds: ["lint"] },
      { sessionFlowLimit: 8, hideImportedSessions: true, acceptancePresetIds: [] },
    );
    expect(merged.sessionFlowLimit).toBe(12);
    expect(merged.acceptancePresetIds).toEqual(["lint"]);
    expect(merged.hideImportedSessions).toBe(false);
  });

  it("keeps local acceptance when server empty", () => {
    const merged = mergeProjectViewPrefs(
      { sessionFlowLimit: 8, acceptancePresetIds: [] },
      { sessionFlowLimit: 8, hideImportedSessions: false, acceptancePresetIds: ["a"] },
    );
    expect(merged.acceptancePresetIds).toEqual(["a"]);
  });

  it("detects local migration candidate", () => {
    expect(
      shouldMigrateLocalToServer(
        { acceptancePresetIds: [] },
        { sessionFlowLimit: 8, hideImportedSessions: false, acceptancePresetIds: ["a"] },
      ),
    ).toBe(true);
    expect(
      shouldMigrateLocalToServer(
        { acceptancePresetIds: ["b"] },
        { sessionFlowLimit: 8, hideImportedSessions: false, acceptancePresetIds: ["a"] },
      ),
    ).toBe(false);
  });
});
