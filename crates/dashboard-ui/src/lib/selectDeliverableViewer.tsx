import type { ReactNode } from "react";
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
import { PreviewHtmlViewer } from "@/components/deliverables/viewers/PreviewHtmlViewer";
import { PresentationThumbViewer } from "@/components/deliverables/viewers/PresentationThumbViewer";
import { SpreadsheetViewer } from "@/components/deliverables/viewers/SpreadsheetViewer";
import { VideoViewer } from "@/components/deliverables/viewers/VideoViewer";
import { WorkbookJsonViewer } from "@/components/deliverables/viewers/WorkbookJsonViewer";
import { basename, extension } from "@/lib/pathUtils";
import { inferPreviewPath } from "@/lib/previewPath";

export type DeliverableCardProps = {
  path: string;
  title?: string;
  kind?: string;
  mime?: string;
  projectId?: string;
  previewPath?: string;
  bytes?: number;
};

export type DeliverableViewerProps = DeliverableCardProps & {
  variant?: "compact" | "full";
};

function displayTitle(path: string, title?: string): string {
  const trimmed = title?.trim();
  if (trimmed) return trimmed;
  return basename(path);
}

function hasPreviewSidecar(path: string, previewPath?: string): boolean {
  return Boolean(previewPath?.trim() || inferPreviewPath(path));
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
    case "spreadsheet":
      return "table_chart";
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

export function selectDeliverableViewer({
  path,
  title,
  kind: kindHint,
  mime: mimeHint,
  projectId,
  previewPath,
  bytes,
  variant = "compact",
}: DeliverableViewerProps): ReactNode {
  const resolvedPath = path.trim();
  if (!resolvedPath) return null;

  const resolvedKind = kindForPath(resolvedPath, kindHint);
  const resolvedMime = mimeForPath(resolvedPath, mimeHint);
  const label = displayTitle(resolvedPath, title);
  const ext = extension(resolvedPath);

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
      if (hasPreviewSidecar(resolvedPath, previewPath)) {
        return (
          <PreviewHtmlViewer
            path={resolvedPath}
            title={label}
            projectId={projectId}
            previewPath={previewPath}
            previewSource="sidecar"
            variant={variant}
          />
        );
      }
      return <PdfViewer path={resolvedPath} title={label} projectId={projectId} />;
    case "spreadsheet":
      if (ext === "json") {
        return (
          <WorkbookJsonViewer
            path={resolvedPath}
            title={label}
            projectId={projectId}
            variant={variant}
          />
        );
      }
      return (
        <SpreadsheetViewer
          path={resolvedPath}
          title={label}
          projectId={projectId}
          previewPath={previewPath}
          variant={variant}
        />
      );
    case "presentation":
      if (hasPreviewSidecar(resolvedPath, previewPath) || ext === "html" || ext === "htm") {
        return (
          <PresentationThumbViewer
            path={resolvedPath}
            title={label}
            projectId={projectId}
            previewPath={previewPath}
            variant={variant}
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
    case "document":
      if (["docx", "doc"].includes(ext) && hasPreviewSidecar(resolvedPath, previewPath)) {
        return (
          <PreviewHtmlViewer
            path={resolvedPath}
            title={label}
            projectId={projectId}
            previewPath={previewPath}
            previewSource="sidecar"
            variant={variant}
          />
        );
      }
      if (["docx", "doc", "pptx", "ppt"].includes(ext)) {
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
    case "mindmap":
      return (
        <MindMapViewer
          path={resolvedPath}
          title={label}
          projectId={projectId}
          variant={variant}
        />
      );
    case "report":
      return (
        <PreviewHtmlViewer
          path={resolvedPath}
          title={label}
          projectId={projectId}
          previewSource="self"
          thumbMode="icon"
          variant={variant}
        />
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
