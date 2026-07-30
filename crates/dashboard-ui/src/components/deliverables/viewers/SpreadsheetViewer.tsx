import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { api } from "@/api/client";
import { DeliverableCompactShell } from "@/components/deliverables/DeliverableCompactShell";
import { DeliverablePanelShell } from "@/components/deliverables/DeliverablePanelShell";
import { PreviewHtmlViewer } from "@/components/deliverables/viewers/PreviewHtmlViewer";
import { SpreadsheetTable } from "@/components/deliverables/viewers/SpreadsheetTable";
import { useT } from "@/i18n/context";
import { parseCsvPreview } from "@/lib/csvParse";
import { extension } from "@/lib/pathUtils";
import { inferPreviewPath } from "@/lib/previewPath";

type Props = {
  path: string;
  title: string;
  projectId: string;
  previewPath?: string;
  variant?: "compact" | "full";
};

export function SpreadsheetViewer({
  path,
  title,
  projectId,
  previewPath,
  variant = "compact",
}: Props) {
  const t = useT();
  const ext = extension(path);
  const isCsv = ext === "csv";
  const resolvedPreview = previewPath?.trim() || inferPreviewPath(path);
  const useHtmlPreview = Boolean(resolvedPreview) && !isCsv;
  const metaLabel = t("conversations.deliverable.spreadsheet");

  const content = useQuery({
    queryKey: ["deliverable-spreadsheet", projectId, path, isCsv],
    queryFn: async () => {
      const res = await api.readProjectFs(projectId, path, 512 * 1024);
      return res.file.content ?? "";
    },
    enabled: Boolean(projectId && path && isCsv && !useHtmlPreview),
    staleTime: 60_000,
  });

  const csvPreview = useMemo(
    () => (content.data ? parseCsvPreview(content.data, 200) : { headers: [], rows: [] }),
    [content.data],
  );

  if (useHtmlPreview) {
    return (
      <PreviewHtmlViewer
        path={path}
        title={title}
        projectId={projectId}
        previewPath={previewPath}
        previewSource="sidecar"
        metaLabel={metaLabel}
        variant={variant}
      />
    );
  }

  const dialogBody = (
    <SpreadsheetTable headers={csvPreview.headers} rows={csvPreview.rows} className="p-4" />
  );

  if (variant === "compact") {
    return (
      <DeliverableCompactShell
        path={path}
        projectId={projectId}
        title={title}
        metaLabel={metaLabel}
        cardClassName="deliverable-compact-card deliverable-compact-card--spreadsheet"
        thumb={
          <div className="deliverable-compact-card__thumb" aria-hidden>
            {content.isPending ? (
              <div className="deliverable-compact-card__thumb-placeholder" />
            ) : (
              <SpreadsheetTable
                headers={csvPreview.headers}
                rows={csvPreview.rows}
                variant="thumb"
                maxCols={6}
                maxRows={5}
              />
            )}
          </div>
        }
        dialogBody={dialogBody}
      />
    );
  }

  return (
    <DeliverablePanelShell path={path} projectId={projectId} title={title} metaLabel={metaLabel}>
      {dialogBody}
    </DeliverablePanelShell>
  );
}
