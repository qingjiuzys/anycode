import { kindForPath } from "@/lib/artifactKind";
import { GenericFileCard } from "@/components/deliverables/viewers/GenericFileCard";

type Props = {
  path: string;
  title: string;
  projectId?: string;
  bytes?: number;
};

function officeIcon(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  if (ext === "pptx" || ext === "ppt" || ext === "key" || ext === "odp") return "slideshow";
  if (ext === "xlsx" || ext === "xls" || ext === "csv" || ext === "ods") return "bar_chart";
  return "description";
}

export function OfficeFileCard({ path, title, projectId, bytes }: Props) {
  const kind = kindForPath(path, "document");
  const icon =
    kind === "presentation" ? "slideshow" : kind === "document" ? officeIcon(path) : "description";

  return (
    <GenericFileCard
      path={path}
      title={title}
      icon={icon}
      projectId={projectId}
      bytes={bytes}
    />
  );
}
