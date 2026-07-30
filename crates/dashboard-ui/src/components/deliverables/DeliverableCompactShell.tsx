import { useState, type ReactNode } from "react";
import { DeliverablePreviewDialog } from "@/components/deliverables/DeliverablePreviewDialog";
import { DeliverableFileActions } from "@/components/deliverables/DeliverableFileActions";
import { useDeliverableFileMeta } from "@/hooks/useDeliverableFileMeta";

type Props = {
  path: string;
  projectId: string;
  title: string;
  metaLabel: string;
  thumb: ReactNode;
  dialogBody: ReactNode;
  cardClassName?: string;
  dialogTitle?: string;
  dialogExtra?: ReactNode;
};

export function DeliverableCompactShell({
  path,
  projectId,
  title,
  metaLabel,
  thumb,
  dialogBody,
  cardClassName = "deliverable-compact-card",
  dialogTitle,
  dialogExtra,
}: Props) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const { fileName, downloadUrl } = useDeliverableFileMeta(projectId, path);

  return (
    <>
      <button
        type="button"
        className={cardClassName}
        onClick={() => setDialogOpen(true)}
        aria-label={`${title}，${metaLabel}`}
      >
        {thumb}
        <div className="deliverable-compact-card__body min-w-0">
          <p className="deliverable-compact-card__title">{title}</p>
          <p className="deliverable-compact-card__meta">{metaLabel}</p>
        </div>
      </button>

      <DeliverablePreviewDialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        title={dialogTitle ?? title}
        subtitle={metaLabel}
        wide
      >
        {dialogExtra}
        {dialogBody}
        <div className="px-4 py-3 border-t border-outline-variant/30">
          <DeliverableFileActions
            path={path}
            downloadUrl={downloadUrl}
            downloadName={fileName}
            compact
          />
        </div>
      </DeliverablePreviewDialog>
    </>
  );
}
