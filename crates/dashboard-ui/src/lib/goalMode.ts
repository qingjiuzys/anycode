export const GOAL_AGENT_ID = "goal" as const;

const STORAGE_PREFIX = "anycode-goal-mode:";

export function goalSlashCommand(locale: "zh" | "en"): string {
  return locale === "zh" ? "目标" : "goal";
}

export function isGoalSlashToken(token: string): boolean {
  const t = token.trim().toLowerCase();
  return t === "目标" || t === "goal" || t === "goal-mode";
}

export function loadGoalMode(sessionKey: string | undefined): boolean {
  if (!sessionKey || typeof sessionStorage === "undefined") return false;
  return sessionStorage.getItem(`${STORAGE_PREFIX}${sessionKey}`) === "1";
}

export function saveGoalMode(sessionKey: string | undefined, on: boolean): void {
  if (!sessionKey || typeof sessionStorage === "undefined") return;
  const key = `${STORAGE_PREFIX}${sessionKey}`;
  if (on) sessionStorage.setItem(key, "1");
  else sessionStorage.removeItem(key);
}
