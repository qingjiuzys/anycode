import { useCallback, useState } from "react";
import { Icon } from "@/components/Icon";
import { DeliverableContextMenu } from "@/components/deliverables/DeliverableContextMenu";
import { useDeliverableProject } from "@/components/deliverables/DeliverableProjectContext";
import { useClipboard } from "@/hooks/useClipboard";
import { useT } from "@/i18n/context";
import { resolveDeliverableAbsPath } from "@/lib/deliverablePath";
import { openExternal, openLocalPath, revealInFileManager } from "@/lib/openExternal";
import { extension } from "@/lib/pathUtils";
import { projectFsRawUrl } from "@/lib/projectFsUrl";

type Props = {
  path: string;
  projectId?: string;
  projectRoot?: string | null;
  downloadUrl?: string;
  downloadName?: string;
  copyImageUrl?: string;
  compact?: boolean;
};

export function DeliverableFileActions({
  path,
  projectId: projectIdProp,
  projectRoot: projectRootProp,
  downloadUrl,
  downloadName,
  copyImageUrl,
  compact = false,
}: Props) {
  const t = useT();
  const ctx = useDeliverableProject();
  const projectId = projectIdProp ?? ctx.projectId;
  const projectRoot = projectRootProp ?? ctx.projectRoot;
  const { copy, copyImage, copied, copiedImage } = useClipboard();
  const absPath = resolveDeliverableAbsPath(path, projectRoot);
  const ext = extension(path);
  const isHtml = ext === "html" || ext === "htm";
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);

  const onOpen = useCallback(() => {
    void (async () => {
      try {
        if (isHtml && projectId) {
          await openExternal(projectFsRawUrl(projectId, path));
          return;
        }
        await openLocalPath(absPath);
      } catch (err) {
        try {
          await revealInFileManager(absPath);
        } catch {
          window.alert(
            err instanceof Error ? err.message : t("conversations.openInFinderFailed"),
          );
        }
      }
    })();
  }, [absPath, isHtml, path, projectId, t]);

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

  const btnClass = compact ? "dw-btn-ghost text-xs py-1 px-2" : "dw-btn-secondary text-xs";

  return (
    <div
      className={`flex flex-wrap items-center gap-1.5 ${compact ? "" : "mt-3"}`}
      onContextMenu={(event) => {
        event.preventDefault();
        event.stopPropagation();
        setMenu({ x: event.clientX, y: event.clientY });
      }}
    >
      <button type="button" className={btnClass} onClick={onOpen}>
        <Icon name="folder_open" size={14} className="inline mr-1" />
        {t("conversations.deliverable.open")}
      </button>
      <button type="button" className={btnClass} onClick={onCopyPath}>
        <Icon name="content_copy" size={14} className="inline mr-1" />
        {copied ? t("common.copied") : t("conversations.deliverable.copyPath")}
      </button>
      {copyImageUrl ? (
        <button
          type="button"
          className={btnClass}
          onClick={() => void copyImage(copyImageUrl)}
        >
          <Icon name="image" size={14} className="inline mr-1" />
          {copiedImage ? t("common.copied") : t("conversations.deliverable.copyImage")}
        </button>
      ) : null}
      {downloadUrl ? (
        <a
          href={downloadUrl}
          download={downloadName}
          className={`${btnClass} no-underline inline-flex items-center`}
        >
          <Icon name="download" size={14} className="inline mr-1" />
          {t("conversations.deliverable.download")}
        </a>
      ) : null}

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
