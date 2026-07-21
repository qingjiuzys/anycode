import { apiUrl } from "@/api/http";

/** Absolute URL to stream a project file (img/video/pdf/download). */
export function projectFsRawUrl(projectId: string, path: string): string {
  const query = `path=${encodeURIComponent(path)}`;
  return apiUrl(`/api/projects/${encodeURIComponent(projectId)}/fs/raw?${query}`);
}
