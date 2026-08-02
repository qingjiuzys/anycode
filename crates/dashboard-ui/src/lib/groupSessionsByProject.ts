import type { SessionWithProject } from "@/api/types";

export type ProjectGroupOption = {
  id: string;
  name: string;
  root_path?: string;
  updated_at?: string;
};

export type ProjectSessionGroup = {
  id: string;
  name: string;
  sessions: SessionWithProject[];
};

function maxTimestamp(...values: Array<string | undefined | null>): string {
  let best = "";
  for (const value of values) {
    const v = value?.trim();
    if (!v) continue;
    if (!best || v.localeCompare(best) > 0) {
      best = v;
    }
  }
  return best;
}

/**
 * Last activity timestamp of a session for sidebar ordering.
 * Uses the end time once the session finished (so a just-closed session
 * stays at the top instead of falling back to its original start time),
 * and the start time while it is still running.
 */
export function sessionLastActiveAt(session: SessionWithProject): string {
  return session.ended_at ?? session.started_at;
}

/** Latest activity for sidebar ordering: newest session + project touch time. */
export function projectGroupActivityAt(
  group: ProjectSessionGroup,
  projectUpdatedAt?: string,
): string {
  const latestSessionAt = group.sessions[0]
    ? sessionLastActiveAt(group.sessions[0])
    : undefined;
  const running = group.sessions.some((s) => s.status === "running");
  if (running) {
    return maxTimestamp(projectUpdatedAt, latestSessionAt);
  }
  return maxTimestamp(latestSessionAt, projectUpdatedAt);
}

export function groupSessionsByProject(
  projectOptions: ProjectGroupOption[],
  sessions: SessionWithProject[],
  pinnedIds: ReadonlySet<string> = new Set(),
  opts?: { allowUnknownProjects?: boolean },
): ProjectSessionGroup[] {
  // When the project catalog is loaded, ignore sessions whose project was
  // archived/removed so they don't resurrect sidebar groups.
  const allowUnknown =
    opts?.allowUnknownProjects ?? projectOptions.length === 0;
  const projectUpdatedAt = new Map(
    projectOptions.map((project) => [project.id, project.updated_at]),
  );
  const map = new Map<string, ProjectSessionGroup>();

  for (const project of projectOptions) {
    map.set(project.id, { id: project.id, name: project.name, sessions: [] });
  }

  for (const session of sessions) {
    const existing = map.get(session.project_id);
    if (existing) {
      existing.sessions.push(session);
    } else if (allowUnknown) {
      map.set(session.project_id, {
        id: session.project_id,
        name: session.project_name,
        sessions: [session],
      });
    }
  }

  // Running (active) sessions float to the top of their project group, then
  // last active first (ended_at ?? started_at). This keeps a session you just
  // finished chatting in near the top instead of falling back to its original
  // start time the moment it stops running.
  const sortByActive = (a: SessionWithProject, b: SessionWithProject) => {
    const aRunning = a.status === "running" ? 1 : 0;
    const bRunning = b.status === "running" ? 1 : 0;
    if (aRunning !== bRunning) return bRunning - aRunning;
    return sessionLastActiveAt(b).localeCompare(sessionLastActiveAt(a));
  };

  for (const group of map.values()) {
    group.sessions.sort(sortByActive);
  }

  const groups = [...map.values()].sort((a, b) => {
    const aPinned = pinnedIds.has(a.id);
    const bPinned = pinnedIds.has(b.id);
    if (aPinned !== bPinned) {
      return aPinned ? -1 : 1;
    }
    const aAt = projectGroupActivityAt(a, projectUpdatedAt.get(a.id));
    const bAt = projectGroupActivityAt(b, projectUpdatedAt.get(b.id));
    if (aAt !== bAt) {
      return bAt.localeCompare(aAt);
    }
    return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
  });

  return groups;
}

export function defaultCollapsedProjectIds(
  groups: ProjectSessionGroup[],
  expandedCount = 2,
): Set<string> {
  return new Set(groups.slice(expandedCount).map((group) => group.id));
}
