import { useCallback } from "react";
import { Icon } from "@/components/Icon";
import { useClipboard } from "@/hooks/useClipboard";
import { useT } from "@/i18n/context";
import { revealInFileManager } from "@/lib/openExternal";

type Props = {
  path: string;
  downloadUrl?: string;
  downloadName?: string;
  copyImageUrl?: string;
  compact?: boolean;
};

export function DeliverableFileActions({
  path,
  downloadUrl,
  downloadName,
  copyImageUrl,
  compact = false,
}: Props) {
  const t = useT();
  const { copy, copyImage, copied, copiedImage } = useClipboard();

  const onReveal = useCallback(() => {
    void revealInFileManager(path).catch((err) => {
      window.alert(
        err instanceof Error ? err.message : t("conversations.openInFinderFailed"),
      );
    });
  }, [path, t]);

  const btnClass = compact ? "dw-btn-ghost text-xs py-1 px-2" : "dw-btn-secondary text-xs";

  return (
    <div className={`flex flex-wrap items-center gap-1.5 ${compact ? "" : "mt-3"}`}>
      <button type="button" className={btnClass} onClick={onReveal}>
        <Icon name="folder_open" size={14} className="inline mr-1" />
        {t("conversations.deliverable.open")}
      </button>
      <button type="button" className={btnClass} onClick={() => void copy(path)}>
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
    </div>
  );
}
