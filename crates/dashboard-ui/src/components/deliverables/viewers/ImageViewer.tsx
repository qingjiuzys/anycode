import { useCallback, useState } from "react";
import { Icon } from "@/components/Icon";
import { DeliverableFileActions } from "@/components/deliverables/DeliverableFileActions";
import { useT } from "@/i18n/context";
import { projectFsRawUrl } from "@/lib/projectFsUrl";

type Props = {
  path: string;
  title: string;
  projectId: string;
  previewPath?: string;
};

export function ImageViewer({ path, title, projectId, previewPath }: Props) {
  const t = useT();
  const [lightbox, setLightbox] = useState(false);
  const displayPath = previewPath?.trim() || path;
  const rawUrl = projectFsRawUrl(projectId, displayPath);
  const fileName = path.split(/[/\\]/).pop() ?? path;

  const closeLightbox = useCallback(() => setLightbox(false), []);

  return (
    <div className="glass-panel rounded-xl border border-outline-variant/40 overflow-hidden">
      <div className="px-4 py-2 border-b border-outline-variant/30 flex items-center justify-between gap-2">
        <p className="m-0 text-sm font-medium truncate">{title}</p>
      </div>
      <div className="relative bg-surface-container-low group">
        <img
          src={rawUrl}
          alt={title}
          className="w-full max-h-[min(420px,60vh)] object-contain bg-surface-container-lowest select-auto"
          loading="lazy"
        />
        <button
          type="button"
          className="absolute top-2 right-2 dw-btn-ghost text-xs py-1 px-2 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity bg-surface-container-lowest/90"
          onClick={() => setLightbox(true)}
          aria-label={t("conversations.deliverable.preview")}
        >
          <Icon name="open_in_new" size={16} />
        </button>
      </div>
      <div className="px-4 pb-3">
        <DeliverableFileActions
          path={path}
          downloadUrl={projectFsRawUrl(projectId, path)}
          downloadName={fileName}
          copyImageUrl={rawUrl}
          compact
        />
      </div>

      {lightbox ? (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/80 p-4"
          role="dialog"
          aria-modal
          aria-label={title}
          onClick={closeLightbox}
        >
          <button
            type="button"
            className="absolute top-4 right-4 dw-btn-ghost text-on-surface"
            onClick={closeLightbox}
            aria-label={t("common.close")}
          >
            <Icon name="close" size={20} />
          </button>
          <img
            src={rawUrl}
            alt={title}
            className="max-w-full max-h-full object-contain select-auto"
            onClick={(event) => event.stopPropagation()}
          />
        </div>
      ) : null}
    </div>
  );
}
