export type ProjectViewPrefs = {
  sessionFlowLimit: number;
  hideImportedSessions: boolean;
  /** Preset ids treated as acceptance (required) gates when running from config */
  acceptancePresetIds: string[];
};

const DEFAULTS: ProjectViewPrefs = {
  sessionFlowLimit: 8,
  hideImportedSessions: false,
  acceptancePresetIds: [],
};

function storageKey(projectId: string) {
  return `anycode-project-view-${projectId}`;
}

export function loadProjectViewPrefs(projectId: string): ProjectViewPrefs {
  try {
    const raw = localStorage.getItem(storageKey(projectId));
    if (!raw) return { ...DEFAULTS };
    const parsed = JSON.parse(raw) as Partial<ProjectViewPrefs>;
    return {
      sessionFlowLimit: clampLimit(parsed.sessionFlowLimit ?? DEFAULTS.sessionFlowLimit),
      hideImportedSessions: Boolean(parsed.hideImportedSessions),
      acceptancePresetIds: Array.isArray(parsed.acceptancePresetIds)
        ? parsed.acceptancePresetIds.filter((x): x is string => typeof x === "string")
        : DEFAULTS.acceptancePresetIds,
    };
  } catch {
    return { ...DEFAULTS };
  }
}

export function saveProjectViewPrefs(projectId: string, prefs: ProjectViewPrefs) {
  localStorage.setItem(
    storageKey(projectId),
    JSON.stringify({
      sessionFlowLimit: clampLimit(prefs.sessionFlowLimit),
      hideImportedSessions: prefs.hideImportedSessions,
      acceptancePresetIds: prefs.acceptancePresetIds,
    }),
  );
}

function clampLimit(n: number): number {
  if (!Number.isFinite(n)) return 8;
  return Math.min(20, Math.max(3, Math.round(n)));
}
