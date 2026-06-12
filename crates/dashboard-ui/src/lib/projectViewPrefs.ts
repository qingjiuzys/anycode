export type ProjectViewPrefs = {
  sessionFlowLimit: number;
  hideImportedSessions: boolean;
  /** Preset ids treated as acceptance (required) gates when running from config */
  acceptancePresetIds: string[];
};

export type ApiProjectViewPrefs = ProjectViewPrefs;

const DEFAULTS: ProjectViewPrefs = {
  sessionFlowLimit: 8,
  hideImportedSessions: false,
  acceptancePresetIds: [],
};

function storageKey(projectId: string) {
  return `anycode-project-view-${projectId}`;
}

export function clampSessionFlowLimit(n: number): number {
  if (!Number.isFinite(n)) return 8;
  return Math.min(20, Math.max(3, Math.round(n)));
}

export function normalizeProjectViewPrefs(
  partial: Partial<ProjectViewPrefs> | null | undefined,
): ProjectViewPrefs {
  return {
    sessionFlowLimit: clampSessionFlowLimit(
      partial?.sessionFlowLimit ?? DEFAULTS.sessionFlowLimit,
    ),
    hideImportedSessions: Boolean(partial?.hideImportedSessions),
    acceptancePresetIds: Array.isArray(partial?.acceptancePresetIds)
      ? partial!.acceptancePresetIds.filter((x): x is string => typeof x === "string")
      : DEFAULTS.acceptancePresetIds,
  };
}

export function loadProjectViewPrefs(projectId: string): ProjectViewPrefs {
  try {
    const raw = localStorage.getItem(storageKey(projectId));
    if (!raw) return { ...DEFAULTS };
    return normalizeProjectViewPrefs(JSON.parse(raw) as Partial<ProjectViewPrefs>);
  } catch {
    return { ...DEFAULTS };
  }
}

export function saveProjectViewPrefs(projectId: string, prefs: ProjectViewPrefs) {
  const normalized = normalizeProjectViewPrefs(prefs);
  localStorage.setItem(storageKey(projectId), JSON.stringify(normalized));
  return normalized;
}

/** Merge server prefs over local defaults; local-only fields win when server is empty. */
export function mergeProjectViewPrefs(
  server: Partial<ProjectViewPrefs> | null | undefined,
  local: ProjectViewPrefs,
): ProjectViewPrefs {
  const fromServer = normalizeProjectViewPrefs(server);
  return normalizeProjectViewPrefs({
    sessionFlowLimit: fromServer.sessionFlowLimit,
    hideImportedSessions: fromServer.hideImportedSessions,
    acceptancePresetIds:
      fromServer.acceptancePresetIds.length > 0
        ? fromServer.acceptancePresetIds
        : local.acceptancePresetIds,
  });
}

export function shouldMigrateLocalToServer(
  server: Partial<ProjectViewPrefs> | null | undefined,
  local: ProjectViewPrefs,
): boolean {
  const fromServer = normalizeProjectViewPrefs(server);
  if (local.acceptancePresetIds.length > 0 && fromServer.acceptancePresetIds.length === 0) {
    return true;
  }
  if (
    local.sessionFlowLimit !== DEFAULTS.sessionFlowLimit &&
    fromServer.sessionFlowLimit === DEFAULTS.sessionFlowLimit
  ) {
    return true;
  }
  if (
    local.hideImportedSessions !== DEFAULTS.hideImportedSessions &&
    fromServer.hideImportedSessions === DEFAULTS.hideImportedSessions
  ) {
    return true;
  }
  return false;
}
