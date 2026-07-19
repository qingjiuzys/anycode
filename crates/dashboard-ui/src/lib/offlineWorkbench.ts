const OFFLINE_WORKBENCH_KEY = "anycode.workbench.offline";

/** In-process source of truth; mirrored to localStorage when available. */
let memoryFlag: boolean | null = null;

function hydrateFromStorage(): boolean {
  try {
    if (typeof localStorage !== "undefined") {
      return localStorage.getItem(OFFLINE_WORKBENCH_KEY) === "1";
    }
  } catch {
    /* ignore */
  }
  return false;
}

function persistToStorage(allowed: boolean): void {
  try {
    if (typeof localStorage === "undefined") return;
    if (allowed) {
      localStorage.setItem(OFFLINE_WORKBENCH_KEY, "1");
    } else {
      localStorage.removeItem(OFFLINE_WORKBENCH_KEY);
    }
  } catch {
    /* ignore quota / private mode */
  }
}

/** Local-first entry: skip cloud-link gate until the user signs in. */
export function isOfflineWorkbenchAllowed(): boolean {
  if (memoryFlag === null) {
    memoryFlag = hydrateFromStorage();
  }
  return memoryFlag;
}

export function setOfflineWorkbenchAllowed(allowed: boolean): void {
  memoryFlag = allowed;
  persistToStorage(allowed);
}
