const KEY = "anycode-dashboard-density";

export type Density = "comfortable" | "compact" | "audit";

export function getDensity(): Density {
  const v = localStorage.getItem(KEY);
  if (v === "compact" || v === "audit") return v;
  return "comfortable";
}

export function setDensity(d: Density) {
  localStorage.setItem(KEY, d);
  document.documentElement.dataset.density = d;
}

export function initDensity() {
  document.documentElement.dataset.density = getDensity();
}
