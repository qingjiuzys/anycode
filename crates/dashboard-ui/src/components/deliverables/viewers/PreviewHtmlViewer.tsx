import { Icon } from "@/components/Icon";
import { DeliverableCompactShell } from "@/components/deliverables/DeliverableCompactShell";
import { DeliverableIframePreview } from "@/components/deliverables/DeliverableIframePreview";
import { DeliverablePanelShell } from "@/components/deliverables/DeliverablePanelShell";
import { useT } from "@/i18n/context";
import { resolvePreviewUrl } from "@/lib/previewPath";

type Props = {
  path: string;
  title: string;
  projectId: string;
  previewPath?: string;
  previewSource?: "self" | "sidecar";
  thumbMode?: "iframe" | "icon";
  metaLabel?: string;
  variant?: "compact" | "full";
};

export function PreviewHtmlViewer({
  path,
  title,
  projectId,
  previewPath,
  previewSource = "sidecar",
  thumbMode = "iframe",
  metaLabel,
  variant = "compact",
}: Props) {
  const t = useT();
  const previewUrl = resolvePreviewUrl(projectId, path, previewPath, previewSource);
  const label =
    metaLabel ??
    (previewSource === "self"
      ? t("conversations.deliverable.htmlReport")
      : t("conversations.deliverable.preview"));

  const dialogBody = (
    <DeliverableIframePreview src={previewUrl} title={title} size="dialog" />
  );

  const thumb =
    thumbMode === "icon" ? (
      <div className="deliverable-compact-card__icon" aria-hidden>
        <Icon name="web" size={20} />
      </div>
    ) : (
      <div className="deliverable-compact-card__thumb" aria-hidden>
        <DeliverableIframePreview src={previewUrl} title={title} size="thumb" />
      </div>
    );

  if (variant === "compact") {
    return (
      <DeliverableCompactShell
        path={path}
        projectId={projectId}
        title={title}
        metaLabel={label}
        cardClassName={`deliverable-compact-card ${
          previewSource === "self"
            ? "deliverable-compact-card--report"
            : "deliverable-compact-card--preview-html"
        }`}
        thumb={thumb}
        dialogBody={dialogBody}
      />
    );
  }

  return (
    <DeliverablePanelShell path={path} projectId={projectId} title={title} metaLabel={label}>
      <DeliverableIframePreview src={previewUrl} title={title} size="dialog" />
    </DeliverablePanelShell>
  );
}

/** @deprecated Use PreviewHtmlViewer with previewSource="self" thumbMode="icon" */
export function HtmlReportViewer(props: Omit<Props, "previewSource" | "thumbMode">) {
  return (
    <PreviewHtmlViewer {...props} previewSource="self" thumbMode="icon" />
  );
}
