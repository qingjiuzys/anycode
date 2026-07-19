import type { ReactNode } from "react";

/**
 * Shared list-page toolbar: filters/search on the left, primary actions on the right.
 * Optional `extra` row for chips (trust/risk). Omit slots that a page does not need.
 */
export function ListPageToolbar({
  left,
  actions,
  extra,
  className = "",
}: {
  left?: ReactNode;
  actions?: ReactNode;
  extra?: ReactNode;
  className?: string;
}) {
  return (
    <div className={`dw-list-page-toolbar ${className}`.trim()}>
      <div className="dw-list-page-toolbar__row">
        {left != null && <div className="dw-list-page-toolbar__left">{left}</div>}
        {actions != null && <div className="dw-list-page-toolbar__actions">{actions}</div>}
      </div>
      {extra != null && <div className="dw-list-page-toolbar__extra">{extra}</div>}
    </div>
  );
}
