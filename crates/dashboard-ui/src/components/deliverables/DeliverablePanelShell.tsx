import type { ReactNode } from "react";
import { DeliverableFileActions } from "@/components/deliverables/DeliverableFileActions";
import { useDeliverableFileMeta } from "@/hooks/useDeliverableFileMeta";

type Props = {
  path: string;
  projectId: string;
  title: string;
  metaLabel?: string;
  headerExtra?: ReactNode;
  children: ReactNode;
};

export function DeliverablePanelShell({
  path,
  projectId,
  title,
  metaLabel,
  headerExtra,
  children,
}: Props) {
  const { fileName, downloadUrl } = useDeliverableFileMeta(projectId, path);

  return (
    <div className="deliverable-panel-card rounded-xl border border-outline-variant/30 overflow-hidden bg-white">
      <div className="px-4 py-2 border-b border-outline-variant/20">
        {headerExtra ?? (
          <>
            <p className="m-0 text-sm font-medium truncate">{title}</p>
            {metaLabel ? (
              <p className="m-0 mt-0.5 text-[11px] text-secondary">{metaLabel}</p>
            ) : null}
          </>
        )}
      </div>
      {children}
      <div className="px-4 pb-3">
        <DeliverableFileActions
          path={path}
          downloadUrl={downloadUrl}
          downloadName={fileName}
          compact
        />
      </div>
    </div>
  );
}
