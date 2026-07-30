import { useState } from "react";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";
import { tableToCsv, type MarkdownTable } from "@/lib/markdownTable";

type Props = {
  table: MarkdownTable;
};

export function TableCard({ table }: Props) {
  const t = useT();
  const [collapsed, setCollapsed] = useState(false);
  const colCount = Math.max(table.headers.length, ...table.rows.map((row) => row.length), 0);
  const rowCount = table.rows.length + (table.headers.length > 0 ? 1 : 0);

  const copyCsv = async () => {
    try {
      await navigator.clipboard.writeText(tableToCsv(table));
    } catch {
      /* ignore */
    }
  };

  return (
    <div className="dw-table-card">
      <div className="dw-table-card__header">
        <div className="min-w-0">
          <p className="dw-table-card__title">{t("conversations.tableCard.title")}</p>
          <p className="dw-table-card__meta">
            {t("conversations.tableCard.meta")
              .replace("{rows}", String(rowCount))
              .replace("{cols}", String(colCount))}
          </p>
        </div>
        <div className="dw-table-card__actions">
          <button type="button" className="dw-btn-ghost text-xs py-1 px-2" onClick={copyCsv}>
            <Icon name="content_copy" size={14} />
            {t("conversations.tableCard.copyCsv")}
          </button>
          <button
            type="button"
            className="dw-btn-ghost text-xs py-1 px-2"
            onClick={() => setCollapsed((value) => !value)}
            aria-expanded={!collapsed}
          >
            <Icon name={collapsed ? "expand_more" : "expand_less"} size={14} />
            {collapsed ? t("conversations.tableCard.expand") : t("conversations.tableCard.collapse")}
          </button>
        </div>
      </div>
      {!collapsed ? (
        <div className="dw-table-card__body">
          <table className="dw-table-card__table">
            {table.headers.length > 0 ? (
              <thead>
                <tr>
                  {table.headers.map((cell, index) => (
                    <th key={`h-${index}`}>{cell}</th>
                  ))}
                </tr>
              </thead>
            ) : null}
            <tbody>
              {table.rows.map((row, rowIndex) => (
                <tr key={`r-${rowIndex}`}>
                  {Array.from({ length: colCount }, (_, colIndex) => (
                    <td key={`c-${rowIndex}-${colIndex}`}>{row[colIndex] ?? ""}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
    </div>
  );
}
