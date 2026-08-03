import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";
import {
  markPlanBuilt,
  planAwaitingBuild,
  planTreeExecutionStarted,
  readPlanBuiltAt,
} from "./planBuildState";

function mockLocalStorage() {
  const store = new Map<string, string>();
  vi.stubGlobal("window", {});
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

describe("planBuildState", () => {
  beforeEach(() => {
    mockLocalStorage();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("detects awaiting build until marked built", () => {
    const tree = {
      roots: [{ id: "r", title: "Root", status: "pending" as const, children: [] }],
    };
    expect(planAwaitingBuild(tree, "2026-01-01T00:00:00Z", "sess-1")).toBe(true);
    markPlanBuilt("sess-1", "2026-01-01T00:00:00Z");
    expect(readPlanBuiltAt("sess-1")).toBe("2026-01-01T00:00:00Z");
    expect(planAwaitingBuild(tree, "2026-01-01T00:00:00Z", "sess-1")).toBe(false);
  });

  it("treats in-progress nodes as execution started", () => {
    const tree = {
      roots: [{ id: "r", title: "Root", status: "in_progress" as const, children: [] }],
    };
    expect(planTreeExecutionStarted(tree)).toBe(true);
    expect(planAwaitingBuild(tree, "t1", "sess-2")).toBe(false);
  });
});
