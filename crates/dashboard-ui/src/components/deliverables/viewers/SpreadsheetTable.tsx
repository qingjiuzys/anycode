type Props = {
  headers: string[];
  rows: string[][];
  className?: string;
  variant?: "full" | "thumb";
  maxCols?: number;
  maxRows?: number;
};

export function SpreadsheetTable({
  headers,
  rows,
  className = "",
  variant = "full",
  maxCols,
  maxRows,
}: Props) {
  if (headers.length === 0 && rows.length === 0) {
    return variant === "thumb" ? (
      <div className="deliverable-compact-card__thumb-placeholder" />
    ) : null;
  }

  const cols = headers.length > 0 ? headers : (rows[0] ?? []);
  const displayHeaders = maxCols ? headers.slice(0, maxCols) : headers;
  const displayRows = maxRows ? rows.slice(0, maxRows) : rows;
  const tableClass =
    variant === "thumb"
      ? "deliverable-spreadsheet-thumb__table"
      : "deliverable-spreadsheet-table";
  const wrapClass =
    variant === "thumb"
      ? "deliverable-spreadsheet-thumb overflow-auto"
      : `deliverable-spreadsheet-table-wrap ${className}`.trim();

  return (
    <div className={wrapClass}>
      <table className={tableClass}>
        {displayHeaders.length > 0 ? (
          <thead>
            <tr>
              {displayHeaders.map((cell, index) => (
                <th key={`h-${index}`}>{cell}</th>
              ))}
            </tr>
          </thead>
        ) : null}
        <tbody>
          {displayRows.map((row, rowIndex) => (
            <tr key={`r-${rowIndex}`}>
              {(headers.length > 0 ? row : cols).map((_, colIndex) => {
                if (maxCols && colIndex >= maxCols) return null;
                return <td key={`c-${rowIndex}-${colIndex}`}>{row[colIndex] ?? ""}</td>;
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
