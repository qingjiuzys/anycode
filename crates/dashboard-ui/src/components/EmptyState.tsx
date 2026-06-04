import type { ReactNode } from "react";
import { Icon } from "@/components/Icon";

export function EmptyState({
  title,
  description,
  icon = "inbox",
  actions,
  compact,
}: {
  title: string;
  description?: string;
  icon?: string;
  actions?: ReactNode;
  compact?: boolean;
}) {
  return (
    <div className={compact ? "dw-empty-compact" : "dw-empty"}>
      <Icon name={icon} size={compact ? 32 : 40} className="text-outline mb-3" />
      <h3 className={`font-semibold text-on-surface m-0 ${compact ? "text-sm" : "text-base"}`}>
        {title}
      </h3>
      {description && (
        <p className={`text-secondary mt-2 max-w-md ${compact ? "text-xs" : "text-sm"}`}>
          {description}
        </p>
      )}
      {actions && <div className="mt-4 flex flex-wrap items-center justify-center gap-2">{actions}</div>}
    </div>
  );
}
