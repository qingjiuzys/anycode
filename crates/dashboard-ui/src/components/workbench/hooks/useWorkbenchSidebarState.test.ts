import { describe, it, expect, beforeEach, vi } from "vitest";
import { resetWorkbenchSidebarStateCache } from "./useWorkbenchSidebarState";

const STORAGE_KEY = "anycode-workbench-sidebar";

describe("useWorkbenchSidebarState storage", () => {
  beforeEach(() => {
    const store = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => {
        store.set(k, v);
      },
      removeItem: (k: string) => {
        store.delete(k);
      },
    });
    resetWorkbenchSidebarStateCache();
    store.clear();
  });

  it("persists expanded state", () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ expanded: true, activeTab: "terminal", panelWidth: 300 }),
    );
    const raw = localStorage.getItem(STORAGE_KEY);
    expect(raw).toBeTruthy();
    const parsed = JSON.parse(raw!) as { expanded: boolean; activeTab: string };
    expect(parsed.expanded).toBe(true);
    expect(parsed.activeTab).toBe("terminal");
  });
});
