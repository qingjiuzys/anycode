import { basename } from "@/lib/pathUtils";
import { projectFsRawUrl } from "@/lib/projectFsUrl";

export function useDeliverableFileMeta(projectId: string, path: string) {
  const fileName = basename(path);
  const downloadUrl = projectFsRawUrl(projectId, path);
  return { fileName, downloadUrl };
}
