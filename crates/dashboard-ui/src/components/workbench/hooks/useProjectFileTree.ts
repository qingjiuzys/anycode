import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";

export function useProjectFsList(
  projectId: string | null | undefined,
  path: string,
  opts?: { enabled?: boolean },
) {
  return useQuery({
    queryKey: ["workbench-fs-list", projectId, path],
    queryFn: () => api.listProjectFs(projectId!, path),
    enabled: Boolean(projectId) && (opts?.enabled ?? true),
    staleTime: 5_000,
  });
}

export function useProjectFsRead(
  projectId: string | null | undefined,
  filePath: string | null,
) {
  return useQuery({
    queryKey: ["workbench-fs-read", projectId, filePath],
    queryFn: () => api.readProjectFs(projectId!, filePath!),
    enabled: Boolean(projectId && filePath),
    staleTime: 10_000,
  });
}
