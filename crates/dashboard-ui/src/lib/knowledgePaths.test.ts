import { describe, expect, it } from "vitest";
import {
  knowledgeIndexStatus,
  parseKnowledgePaths,
  pathsEqual,
  validateKnowledgePath,
} from "./knowledgePaths";

describe("knowledgePaths", () => {
  it("parses and dedupes lines", () => {
    expect(parseKnowledgePaths("docs/\n\nreports/\ndocs/")).toEqual(["docs/", "reports/"]);
  });

  it("validates path rules", () => {
    expect(validateKnowledgePath("docs/")).toBeNull();
    expect(validateKnowledgePath("/abs")).toBe("absolute");
    expect(validateKnowledgePath("../x")).toBe("parent");
  });

  it("detects index status", () => {
    expect(knowledgeIndexStatus([], [], 0)).toBe("empty");
    expect(knowledgeIndexStatus(["docs/"], ["docs/"], 0)).toBe("stale");
    expect(knowledgeIndexStatus(["docs/"], ["reports/"], 5)).toBe("stale");
    expect(knowledgeIndexStatus(["docs/"], ["docs/"], 12)).toBe("ready");
  });

  it("compares path lists", () => {
    expect(pathsEqual(["a", "b"], ["b", "a"])).toBe(true);
    expect(pathsEqual(["a"], ["a", "b"])).toBe(false);
  });
});
