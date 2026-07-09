export const HIDE_REPORTS_KEY = "anycode.features.hideReports";
export const SHOW_REPORTS_KEY = "anycode.features.showReports";
export const FEATURE_FLAGS_EVENT = "anycode-feature-flags";

/** Reports nav is hidden by default; enable in Settings → Features or legacy hideReports=0. */
export function isReportsNavHidden(): boolean {
  if (localStorage.getItem(SHOW_REPORTS_KEY) === "1") {
    return false;
  }
  if (localStorage.getItem(HIDE_REPORTS_KEY) === "0") {
    return false;
  }
  return true;
}

export function setReportsNavHidden(hidden: boolean): void {
  if (hidden) {
    localStorage.removeItem(SHOW_REPORTS_KEY);
    localStorage.setItem(HIDE_REPORTS_KEY, "1");
  } else {
    localStorage.removeItem(HIDE_REPORTS_KEY);
    localStorage.setItem(SHOW_REPORTS_KEY, "1");
  }
  window.dispatchEvent(new Event(FEATURE_FLAGS_EVENT));
}
