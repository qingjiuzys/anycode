import { useCallback, useEffect, useState } from "react";
import type { WorkbenchTab } from "@/api/types/workbench";

const STORAGE_KEY = "anycode-workbench-sidebar";

export type WorkbenchSidebarState = {
  expanded: boolean;
  activeTab: WorkbenchTab;
  panelWidth: number;
};

const DEFAULT: WorkbenchSidebarState = {
  expanded: false,
  activeTab: "files",
  panelWidth: 280,
};

function readState(): WorkbenchSidebarState {
  if (typeof window === "undefined") return DEFAULT;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT;
    const parsed = JSON.parse(raw) as Partial<WorkbenchSidebarState>;
    return {
      expanded: parsed.expanded ?? DEFAULT.expanded,
      activeTab: parsed.activeTab ?? DEFAULT.activeTab,
      panelWidth: parsed.panelWidth ?? DEFAULT.panelWidth,
    };
  } catch {
    return DEFAULT;
  }
}

function writeState(state: WorkbenchSidebarState) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

export function useWorkbenchSidebarState() {
  const [state, setState] = useState<WorkbenchSidebarState>(readState);

  useEffect(() => {
    writeState(state);
  }, [state]);

  const selectTab = useCallback((tab: WorkbenchTab) => {
    setState((prev) => {
      if (prev.expanded && prev.activeTab === tab) {
        return { ...prev, expanded: false };
      }
      return { ...prev, expanded: true, activeTab: tab };
    });
  }, []);

  const setExpanded = useCallback((expanded: boolean) => {
    setState((prev) => ({ ...prev, expanded }));
  }, []);

  const setPanelWidth = useCallback((panelWidth: number) => {
    setState((prev) => ({ ...prev, panelWidth: Math.min(480, Math.max(240, panelWidth)) }));
  }, []);

  const openTab = useCallback((tab: WorkbenchTab) => {
    setState((prev) => ({ ...prev, expanded: true, activeTab: tab }));
  }, []);

  return { ...state, selectTab, setExpanded, setPanelWidth, openTab };
}

export function resetWorkbenchSidebarStateCache(): void {
  if (typeof window === "undefined") return;
  localStorage.removeItem(STORAGE_KEY);
}
