import type { ReactNode } from "react";
import { ControlCenterLink } from "@/components/control-center/ControlCenterLink";
import { Icon } from "@/components/Icon";

export type BreadcrumbItem = { label: string; to?: string };

export function PageHeader({
  title,
  subtitle,
  meta,
  actions,
  breadcrumbs,
}: {
  title: ReactNode;
  subtitle?: string;
  meta?: ReactNode;
  actions?: ReactNode;
  breadcrumbs?: BreadcrumbItem[];
}) {
  return (
    <div className="dw-page-header flex flex-col gap-2">
      {breadcrumbs && breadcrumbs.length > 0 && (
        <nav
          className="dw-page-header-breadcrumbs flex flex-wrap items-center gap-1 text-xs text-secondary"
          aria-label="Breadcrumb"
        >
          {breadcrumbs.map((item, i) => (
            <span key={`${item.label}-${i}`} className="inline-flex items-center gap-1">
              {i > 0 && <Icon name="chevron_right" size={14} className="text-outline" />}
              {item.to ? (
                <ControlCenterLink to={item.to} className="no-underline hover:underline">
                  {item.label}
                </ControlCenterLink>
              ) : (
                <span className="text-on-surface-variant">{item.label}</span>
              )}
            </span>
          ))}
        </nav>
      )}
      <div className="dw-page-header-row flex flex-col md:flex-row md:items-end justify-between gap-4">
        <div className="dw-page-header-intro flex flex-col gap-1 min-w-0">
          <h1 className="text-2xl font-bold text-on-surface tracking-tight">{title}</h1>
          {subtitle && <p className="text-sm text-secondary">{subtitle}</p>}
          {meta && (
            <div className="flex flex-wrap items-center gap-2 text-xs font-code text-secondary mt-1">
              {meta}
            </div>
          )}
        </div>
        {actions && (
          <div className="dw-page-header-actions flex items-center gap-2 flex-wrap shrink-0">
            {actions}
          </div>
        )}
      </div>
    </div>
  );
}
