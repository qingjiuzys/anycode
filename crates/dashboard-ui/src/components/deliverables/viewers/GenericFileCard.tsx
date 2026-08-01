import { useCallback, useState } from "react";
import { Icon } from "@/components/Icon";
import { DeliverableContextMenu } from "@/components/deliverables/DeliverableContextMenu";
import { DeliverableFileActions } from "@/components/deliverables/DeliverableFileActions";
import { useDeliverableProject } from "@/components/deliverables/DeliverableProjectContext";
import { useClipboard } from "@/hooks/useClipboard";
import { useT } from "@/i18n/context";
import { resolveDeliverableAbsPath } from "@/lib/deliverablePath";
import { revealInFileManager } from "@/lib/openExternal";
import { projectFsRawUrl } from "@/lib/projectFsUrl";

type Props = {
  path: string;
  title: string;
  icon: string;
  projectId?: string;
  projectRoot?: string | null;
  bytes?: number;
};

function formatBytes(bytes?: number): string | null {
  if (bytes === undefined || !Number.isFinite(bytes) || bytes < 0) return null;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function GenericFileCard({
  path,
  title,
  icon,
  projectId: projectIdProp,
  projectRoot: projectRootProp,
  bytes,
}: Props) {
  const t = useT();
  const ctx = useDeliverableProject();
  const projectId = projectIdProp ?? ctx.projectId;
  const projectRoot = projectRootProp ?? ctx.projectRoot;
  const { copy } = useClipboard();
  const fileName = path.split(/[/\\]/).pop() ?? path;
  const rawUrl = projectId ? projectFsRawUrl(projectId, path) : undefined;
  const sizeLabel = formatBytes(bytes);
  const absPath = resolveDeliverableAbsPath(path, projectRoot);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);

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
    <div
      className="deliverable-file-card glass-panel rounded-xl border border-outline-variant/40 p-3"
      onContextMenu={(event) => {
        event.preventDefault();
        event.stopPropagation();
        setMenu({ x: event.clientX, y: event.clientY });
      }}
    >
      <div className="flex items-start gap-3 min-w-0">
        <div className="shrink-0 rounded-lg bg-surface-container-high p-2 text-secondary">
          <Icon name={icon} size={20} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="m-0 text-sm font-medium text-on-surface truncate">{title}</p>
          <p className="m-0 mt-0.5 text-[11px] text-secondary font-code truncate" title={absPath || path}>
            {path}
          </p>
          {sizeLabel ? (
            <p className="m-0 mt-0.5 text-[11px] text-secondary">{sizeLabel}</p>
          ) : null}
        </div>
      </div>
      <DeliverableFileActions
        path={path}
        projectId={projectId}
        projectRoot={projectRoot}
        downloadUrl={rawUrl}
        downloadName={fileName}
        compact
      />
      {menu ? (
        <DeliverableContextMenu
          x={menu.x}
          y={menu.y}
          onClose={() => setMenu(null)}
          onReveal={onReveal}
          onCopyPath={onCopyPath}
        />
      ) : null}
    </div>
  );
}
