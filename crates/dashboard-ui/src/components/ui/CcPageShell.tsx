import type { ReactNode } from "react";

/**
 * Control-center / feature page shell: pinned header, body scrolls independently.
 *
 * Prefer this for every CC feature page (lists + details). Settings uses
 * `dw-settings-page` instead (same scroll contract). Home hero is exempt.
 */
export function CcPageShell({
  header,
  children,
  className = "",
}: {
  header: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={`dw-cc-page ${className}`.trim()}>
      <div className="dw-cc-page-header">{header}</div>
      <div className="dw-cc-page-body">{children}</div>
    </div>
  );
}
