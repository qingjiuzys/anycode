import { useCallback, useEffect, useState } from "react";
import {
  loadProjectViewPrefs,
  saveProjectViewPrefs,
  type ProjectViewPrefs,
} from "@/lib/projectViewPrefs";

export function useProjectViewPrefs(projectId: string) {
  const [prefs, setPrefs] = useState<ProjectViewPrefs>(() => loadProjectViewPrefs(projectId));

  useEffect(() => {
    setPrefs(loadProjectViewPrefs(projectId));
  }, [projectId]);

  const update = useCallback(
    (patch: Partial<ProjectViewPrefs>) => {
      setPrefs((prev) => {
        const next = { ...prev, ...patch };
        saveProjectViewPrefs(projectId, next);
        return next;
      });
    },
    [projectId],
  );

  return { prefs, update };
}
