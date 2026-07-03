/** Human-readable audit action labels (maps backend `event_type` → i18n key). */
export function auditActionLabel(action: string, t: (key: string) => string): string {
  const key = `audit.actions.${action}`;
  const label = t(key);
  return label === key ? action.replaceAll("_", " ") : label;
}

export function auditRiskLabel(risk: string, t: (key: string) => string): string {
  const key = `status.${risk.toLowerCase()}`;
  const label = t(key);
  return label === key ? risk : label;
}
