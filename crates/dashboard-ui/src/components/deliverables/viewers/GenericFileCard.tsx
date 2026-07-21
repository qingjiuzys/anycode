import { Icon } from "@/components/Icon";
import { DeliverableFileActions } from "@/components/deliverables/DeliverableFileActions";
import { projectFsRawUrl } from "@/lib/projectFsUrl";

type Props = {
  path: string;
  title: string;
  icon: string;
  projectId?: string;
  bytes?: number;
};

function formatBytes(bytes?: number): string | null {
  if (bytes === undefined || !Number.isFinite(bytes) || bytes < 0) return null;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function GenericFileCard({ path, title, icon, projectId, bytes }: Props) {
  const fileName = path.split(/[/\\]/).pop() ?? path;
  const rawUrl = projectId ? projectFsRawUrl(projectId, path) : undefined;
  const sizeLabel = formatBytes(bytes);

  return (
    <div className="glass-panel rounded-xl border border-outline-variant/40 p-4">
      <div className="flex items-start gap-3 min-w-0">
        <div className="shrink-0 rounded-lg bg-surface-container-high p-2.5 text-secondary">
          <Icon name={icon} size={22} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="m-0 text-sm font-medium text-on-surface truncate">{title}</p>
          <p className="m-0 mt-1 text-[11px] text-secondary font-code truncate">{path}</p>
          {sizeLabel ? (
            <p className="m-0 mt-1 text-[11px] text-secondary">{sizeLabel}</p>
          ) : null}
        </div>
      </div>
      <DeliverableFileActions
        path={path}
        downloadUrl={rawUrl}
        downloadName={fileName}
      />
    </div>
  );
}
