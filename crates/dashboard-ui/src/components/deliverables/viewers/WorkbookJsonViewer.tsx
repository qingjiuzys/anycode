import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { api } from "@/api/client";
import { DeliverableCompactShell } from "@/components/deliverables/DeliverableCompactShell";
import { DeliverablePanelShell } from "@/components/deliverables/DeliverablePanelShell";
import { SpreadsheetTable } from "@/components/deliverables/viewers/SpreadsheetTable";
import { useT } from "@/i18n/context";
import { parseWorkbookJson } from "@/lib/workbookJson";

type Props = {
  path: string;
  title: string;
  projectId: string;
  variant?: "compact" | "full";
};

function SheetTabs({
  sheets,
  activeSheet,
  onSelect,
}: {
  sheets: Array<{ name: string }>;
  activeSheet: number;
  onSelect: (index: number) => void;
}) {
  if (sheets.length <= 1) return null;
  return (
    <div className="px-4 pt-3 flex flex-wrap gap-1.5 border-b border-outline-variant/20">
      {sheets.map((sheet, index) => (
        <button
          key={sheet.name}
          type="button"
          className={`dw-btn-ghost text-xs py-1 px-2 ${index === activeSheet ? "bg-surface-container-high" : ""}`}
          onClick={() => onSelect(index)}
        >
          {sheet.name}
        </button>
      ))}
    </div>
  );
}

export function WorkbookJsonViewer({ path, title, projectId, variant = "compact" }: Props) {
  const t = useT();
  const [activeSheet, setActiveSheet] = useState(0);
  const metaLabel = t("conversations.deliverable.workbookJson");

  const content = useQuery({
    queryKey: ["deliverable-workbook-json", projectId, path],
    queryFn: async () => {
      const res = await api.readProjectFs(projectId, path, 1024 * 1024);
      return res.file.content ?? "";
    },
    enabled: Boolean(projectId && path),
    staleTime: 60_000,
  });

  const workbook = useMemo(
    () => (content.data ? parseWorkbookJson(content.data) : null),
    [content.data],
  );

  const sheet = workbook?.sheets[activeSheet];
  const displayTitle = workbook?.title ?? title;

  const tableBody = sheet ? (
    <SpreadsheetTable headers={sheet.headers} rows={sheet.rows} className="p-4" />
  ) : (
    <p className="m-0 p-4 text-sm text-secondary">{t("common.loading")}</p>
  );

  const sheetTabs = workbook ? (
    <SheetTabs sheets={workbook.sheets} activeSheet={activeSheet} onSelect={setActiveSheet} />
  ) : null;

  if (variant === "compact") {
    return (
      <DeliverableCompactShell
        path={path}
        projectId={projectId}
        title={displayTitle}
        metaLabel={metaLabel}
        dialogTitle={displayTitle}
        cardClassName="deliverable-compact-card deliverable-compact-card--spreadsheet"
        thumb={
          <div className="deliverable-compact-card__thumb" aria-hidden>
            {content.isPending || !sheet ? (
              <div className="deliverable-compact-card__thumb-placeholder" />
            ) : (
              <SpreadsheetTable
                headers={sheet.headers}
                rows={sheet.rows}
                variant="thumb"
                maxRows={5}
              />
            )}
          </div>
        }
        dialogExtra={sheetTabs}
        dialogBody={tableBody}
      />
    );
  }

  return (
    <DeliverablePanelShell
      path={path}
      projectId={projectId}
      title={displayTitle}
      metaLabel={metaLabel}
    >
      <>
        {sheetTabs}
        {tableBody}
      </>
    </DeliverablePanelShell>
  );
}
