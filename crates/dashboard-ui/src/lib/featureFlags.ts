export const HIDE_REPORTS_KEY = "anycode.features.hideReports";
export const FEATURE_FLAGS_EVENT = "anycode-feature-flags";

export function isReportsNavHidden(): boolean {
  return localStorage.getItem(HIDE_REPORTS_KEY) === "1";
}

export function setReportsNavHidden(hidden: boolean): void {
  localStorage.setItem(HIDE_REPORTS_KEY, hidden ? "1" : "0");
  window.dispatchEvent(new Event(FEATURE_FLAGS_EVENT));
}
