import { useCallback, useState, type ReactNode } from "react";
import { DeliverableContextMenu } from "@/components/deliverables/DeliverableContextMenu";
import { DeliverablePreviewDialog } from "@/components/deliverables/DeliverablePreviewDialog";
import { DeliverableFileActions } from "@/components/deliverables/DeliverableFileActions";
import { useDeliverableProject } from "@/components/deliverables/DeliverableProjectContext";
import { useDeliverableFileMeta } from "@/hooks/useDeliverableFileMeta";
import { useClipboard } from "@/hooks/useClipboard";
import { useT } from "@/i18n/context";
import { resolveDeliverableAbsPath } from "@/lib/deliverablePath";
import { revealInFileManager } from "@/lib/openExternal";

type Props = {
  path: string;
  projectId: string;
  projectRoot?: string | null;
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
  projectRoot: projectRootProp,
  title,
  metaLabel,
  thumb,
  dialogBody,
  cardClassName = "deliverable-compact-card",
  dialogTitle,
  dialogExtra,
}: Props) {
  const t = useT();
  const ctx = useDeliverableProject();
  const projectRoot = projectRootProp ?? ctx.projectRoot;
  const { copy } = useClipboard();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const { fileName, downloadUrl } = useDeliverableFileMeta(projectId, path);
  const absPath = resolveDeliverableAbsPath(path, projectRoot);

  const onReveal = useCallback(() => {
    void revealInFileManager(absPath).catch((err) => {
      window.alert(
        err instanceof Error ? err.message : t("conversations.openInFinderFailed"),
      );
    });
  }, [absPath, t]);

  const onCopyPath = useCallback(() => {
    void copy(absPath || path);
  }, [absPath, copy, path]);

  return (
    <>
      <button
        type="button"
        className={cardClassName}
        onClick={() => setDialogOpen(true)}
        onContextMenu={(event) => {
          event.preventDefault();
          event.stopPropagation();
          setMenu({ x: event.clientX, y: event.clientY });
        }}
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
            projectId={projectId}
            projectRoot={projectRoot}
            downloadUrl={downloadUrl}
            downloadName={fileName}
            compact
          />
        </div>
      </DeliverablePreviewDialog>

      {menu ? (
        <DeliverableContextMenu
          x={menu.x}
          y={menu.y}
          onClose={() => setMenu(null)}
          onReveal={onReveal}
          onCopyPath={onCopyPath}
        />
      ) : null}
    </>
  );
}
