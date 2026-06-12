import { useCallback, useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import {
  loadProjectViewPrefs,
  mergeProjectViewPrefs,
  normalizeProjectViewPrefs,
  saveProjectViewPrefs,
  shouldMigrateLocalToServer,
  type ProjectViewPrefs,
} from "@/lib/projectViewPrefs";

export function useProjectViewPrefs(projectId: string) {
  const queryClient = useQueryClient();
  const [prefs, setPrefs] = useState<ProjectViewPrefs>(() => loadProjectViewPrefs(projectId));
  const [savedFlash, setSavedFlash] = useState(false);
  const migrateDone = useRef(false);

  const remote = useQuery({
    queryKey: ["project-view-prefs", projectId],
    queryFn: () => api.projectViewPrefs(projectId),
    staleTime: 60_000,
    retry: false,
  });

  const persist = useMutation({
    mutationFn: (next: ProjectViewPrefs) => api.setProjectViewPrefs(projectId, next),
    onSuccess: (data) => {
      const normalized = normalizeProjectViewPrefs(data.view_prefs);
      saveProjectViewPrefs(projectId, normalized);
      setPrefs(normalized);
      queryClient.setQueryData(["project-view-prefs", projectId], data);
    },
  });

  useEffect(() => {
    setPrefs(loadProjectViewPrefs(projectId));
    migrateDone.current = false;
  }, [projectId]);

  useEffect(() => {
    if (!remote.data?.view_prefs) return;
    const local = loadProjectViewPrefs(projectId);
    const merged = mergeProjectViewPrefs(remote.data.view_prefs, local);
    setPrefs(merged);
    saveProjectViewPrefs(projectId, merged);
    if (!migrateDone.current && shouldMigrateLocalToServer(remote.data.view_prefs, local)) {
      migrateDone.current = true;
      void persist.mutateAsync(merged).catch(() => {});
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- migrate once per mount
  }, [projectId, remote.data?.view_prefs]);

  const update = useCallback(
    (patch: Partial<ProjectViewPrefs>) => {
      setPrefs((prev) => {
        const next = normalizeProjectViewPrefs({ ...prev, ...patch });
        saveProjectViewPrefs(projectId, next);
        persist.mutate(next);
        setSavedFlash(true);
        window.setTimeout(() => setSavedFlash(false), 2000);
        return next;
      });
    },
    [persist, projectId],
  );

  return { prefs, update, savedFlash, isSyncing: persist.isPending || remote.isLoading };
}
