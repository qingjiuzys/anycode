import type { ReactNode } from "react";

export function SectionCard({
  title,
  children,
  className,
  action,
  noPadding,
}: {
  title?: string;
  children: ReactNode;
  className?: string;
  action?: ReactNode;
  noPadding?: boolean;
}) {
  return (
    <div className={`dw-section-card ${className ?? ""}`}>
      {title && (
        <div className="dw-section-header">
          <h3 className="dw-section-title">{title}</h3>
          {action}
        </div>
      )}
      <div className={noPadding ? "" : "p-4"}>{children}</div>
    </div>
  );
}
