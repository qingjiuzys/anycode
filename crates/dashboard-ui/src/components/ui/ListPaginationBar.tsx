import { useT } from "@/i18n/context";

type Props = {
  page: number;
  pageCount: number;
  pageSize: number;
  pageSizeOptions?: number[];
  total?: number;
  pageSizeLabel?: string;
  onPageChange: (page: number) => void;
  onPageSizeChange?: (pageSize: number) => void;
  className?: string;
};

const DEFAULT_SIZES = [10, 20, 50];

/** Footer pagination bar for settings / list cards. */
export function ListPaginationBar({
  page,
  pageCount,
  pageSize,
  pageSizeOptions = DEFAULT_SIZES,
  total,
  pageSizeLabel,
  onPageChange,
  onPageSizeChange,
  className = "",
}: Props) {
  const t = useT();
  // Always show when caller manages page size or reports a total (list pages).
  if (pageCount <= 1 && !onPageSizeChange && total == null) return null;

  return (
    <div
      className={`dw-list-pagination ${className}`.trim()}
      role="navigation"
      aria-label={t("common.pagination")}
    >
      <div className="dw-list-pagination__meta">
        {typeof total === "number" ? (
          <span className="text-xs text-secondary tabular-nums">
            {t("common.paginationTotal").replace("{n}", String(total))}
          </span>
        ) : null}
        {onPageSizeChange ? (
          <label className="dw-list-pagination__size">
            <span className="sr-only">{pageSizeLabel ?? t("common.pageSize")}</span>
            <select
              className="dw-input h-8 text-xs"
              value={pageSize}
              aria-label={pageSizeLabel ?? t("common.pageSize")}
              onChange={(e) => onPageSizeChange(Number(e.target.value))}
            >
              {pageSizeOptions.map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
          </label>
        ) : null}
      </div>
      <div className="dw-list-pagination__nav">
        <button
          type="button"
          className="dw-btn-secondary text-xs"
          disabled={page <= 0}
          onClick={() => onPageChange(Math.max(0, page - 1))}
        >
          {t("common.previous")}
        </button>
        <span className="text-xs text-secondary tabular-nums">
          {page + 1} / {Math.max(1, pageCount)}
        </span>
        <button
          type="button"
          className="dw-btn-secondary text-xs"
          disabled={page + 1 >= pageCount}
          onClick={() => onPageChange(Math.min(pageCount - 1, page + 1))}
        >
          {t("common.next")}
        </button>
      </div>
    </div>
  );
}
