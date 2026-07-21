import {
  kindForPath,
  mimeForPath,
  type ArtifactKind,
} from "@/lib/artifactKind";
import { GenericFileCard } from "@/components/deliverables/viewers/GenericFileCard";
import { ImageViewer } from "@/components/deliverables/viewers/ImageViewer";
import { MindMapViewer } from "@/components/deliverables/viewers/MindMapViewer";
import { OfficeFileCard } from "@/components/deliverables/viewers/OfficeFileCard";
import { PdfViewer } from "@/components/deliverables/viewers/PdfViewer";
import { VideoViewer } from "@/components/deliverables/viewers/VideoViewer";

export type DeliverableCardProps = {
  path: string;
  title?: string;
  kind?: string;
  mime?: string;
  projectId?: string;
  previewPath?: string;
  bytes?: number;
};

function displayTitle(path: string, title?: string): string {
  const trimmed = title?.trim();
  if (trimmed) return trimmed;
  return path.split(/[/\\]/).pop() ?? path;
}

function iconForKind(kind: ArtifactKind): string {
  switch (kind) {
    case "image":
      return "image";
    case "video":
      return "movie";
    case "audio":
      return "mic";
    case "pdf":
      return "description";
    case "presentation":
      return "slideshow";
    case "document":
      return "article";
    case "mindmap":
      return "psychology";
    case "report":
      return "description";
    case "media":
      return "image";
    default:
      return "attach_file";
  }
}

export function DeliverableCard({
  path,
  title,
  kind: kindHint,
  mime: mimeHint,
  projectId,
  previewPath,
  bytes,
}: DeliverableCardProps) {
  const resolvedPath = path.trim();
  if (!resolvedPath) return null;

  const resolvedKind = kindForPath(resolvedPath, kindHint);
  const resolvedMime = mimeForPath(resolvedPath, mimeHint);
  const label = displayTitle(resolvedPath, title);

  if (!projectId) {
    return (
      <GenericFileCard
        path={resolvedPath}
        title={label}
        icon={iconForKind(resolvedKind)}
        bytes={bytes}
      />
    );
  }

  switch (resolvedKind) {
    case "image":
    case "media":
      if (resolvedMime.startsWith("video/") || kindForPath(resolvedPath) === "video") {
        return (
          <VideoViewer
            path={resolvedPath}
            title={label}
            projectId={projectId}
            mime={resolvedMime}
          />
        );
      }
      return (
        <ImageViewer
          path={resolvedPath}
          title={label}
          projectId={projectId}
          previewPath={previewPath}
        />
      );
    case "video":
      return (
        <VideoViewer
          path={resolvedPath}
          title={label}
          projectId={projectId}
          mime={resolvedMime}
        />
      );
    case "pdf":
      return <PdfViewer path={resolvedPath} title={label} projectId={projectId} />;
    case "presentation":
      if (previewPath?.trim()) {
        return (
          <ImageViewer
            path={resolvedPath}
            title={label}
            projectId={projectId}
            previewPath={previewPath}
          />
        );
      }
      return (
        <OfficeFileCard
          path={resolvedPath}
          title={label}
          projectId={projectId}
          bytes={bytes}
        />
      );
    case "document": {
      const ext = resolvedPath.split(".").pop()?.toLowerCase() ?? "";
      if (["docx", "doc", "xlsx", "xls", "pptx", "ppt"].includes(ext)) {
        return (
          <OfficeFileCard
            path={resolvedPath}
            title={label}
            projectId={projectId}
            bytes={bytes}
          />
        );
      }
      return (
        <GenericFileCard
          path={resolvedPath}
          title={label}
          icon={iconForKind(resolvedKind)}
          projectId={projectId}
          bytes={bytes}
        />
      );
    }
    case "mindmap":
      return (
        <MindMapViewer path={resolvedPath} title={label} projectId={projectId} />
      );
    default:
      return (
        <GenericFileCard
          path={resolvedPath}
          title={label}
          icon={iconForKind(resolvedKind)}
          projectId={projectId}
          bytes={bytes}
        />
      );
  }
}
