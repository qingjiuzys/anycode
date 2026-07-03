const ACCEPTANCE_DEFAULT_KEY = "anycode.gates.acceptanceDefault";

export function loadAcceptanceGatesDefault(): boolean {
  return localStorage.getItem(ACCEPTANCE_DEFAULT_KEY) === "1";
}

export function saveAcceptanceGatesDefault(enabled: boolean): void {
  localStorage.setItem(ACCEPTANCE_DEFAULT_KEY, enabled ? "1" : "0");
}
