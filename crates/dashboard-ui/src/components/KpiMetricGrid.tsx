import type { ReactNode } from "react";

export type KpiMetric = {
  label: string;
  value: string;
  highlight?: boolean;
};

export function KpiMetricGrid({ metrics }: { metrics: KpiMetric[] }) {
  return (
    <div className="dw-kpi-grid">
      {metrics.map((m) => (
        <div
          key={m.label}
          className={`dw-kpi-metric ${m.highlight ? "dw-kpi-metric--highlight" : ""}`}
        >
          <span className="dw-kpi-metric__label">{m.label}</span>
          <span className="dw-kpi-metric__value">{m.value}</span>
        </div>
      ))}
    </div>
  );
}

export function AnalyticsBlock({
  title,
  action,
  children,
  footer,
}: {
  title: string;
  action?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
}) {
  return (
    <section className="dw-analytics-block">
      <div className="dw-analytics-block__head">
        <h3 className="dw-analytics-block__title">{title}</h3>
        {action}
      </div>
      <div className="dw-analytics-block__body">{children}</div>
      {footer && <div className="dw-analytics-block__footer">{footer}</div>}
    </section>
  );
}
