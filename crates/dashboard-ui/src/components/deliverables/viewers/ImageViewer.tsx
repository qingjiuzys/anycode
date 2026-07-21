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
      <button
        type="button"
        className="block w-full p-0 border-0 bg-surface-container-low cursor-zoom-in"
        onClick={() => setLightbox(true)}
        aria-label={title}
      >
        <img
          src={rawUrl}
          alt={title}
          className="w-full max-h-[min(420px,60vh)] object-contain bg-surface-container-lowest"
          loading="lazy"
        />
      </button>
      <div className="px-4 pb-3">
        <DeliverableFileActions
          path={path}
          downloadUrl={projectFsRawUrl(projectId, path)}
          downloadName={fileName}
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
            className="max-w-full max-h-full object-contain"
            onClick={(event) => event.stopPropagation()}
          />
        </div>
      ) : null}
    </div>
  );
}
