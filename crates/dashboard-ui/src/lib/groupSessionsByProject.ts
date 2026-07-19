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

/** Latest activity for sidebar ordering: newest session + project touch time. */
export function projectGroupActivityAt(
  group: ProjectSessionGroup,
  projectUpdatedAt?: string,
): string {
  const latestSessionAt = group.sessions[0]?.started_at;
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

  const sortByStarted = (a: SessionWithProject, b: SessionWithProject) =>
    b.started_at.localeCompare(a.started_at);

  for (const group of map.values()) {
    group.sessions.sort(sortByStarted);
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
